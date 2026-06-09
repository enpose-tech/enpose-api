# Top-level CMake package for the Enpose API.
#
# Provides ready-to-use targets so a consuming project does not have to copy
# headers or locate the compiled library:
#
#   enpose_api::enpose_api      -- C interface (shared library + C header)
#   enpose_api::enpose_api_cpp  -- header-only C++ wrapper (adds the C++ header)
#
# Use it in one of three ways:
#
#   * add_subdirectory(path/to/enpose-api)
#   * FetchContent_Declare(enpose_api GIT_REPOSITORY ... ) + FetchContent_MakeAvailable
#   * install it (cmake --install) or build a binary package
#     (cmake --build build --target dist) and find_package(enpose_api CONFIG)
#
# Either way: target_link_libraries(your_app PRIVATE enpose_api::enpose_api_cpp)
#
# The shared library is built from the Rust crate in rust/ via cargo, so a
# build-from-source consumer needs a Rust toolchain. The binary package ships a
# prebuilt library for consumers that do not.

cmake_minimum_required(VERSION 3.19)
project(enpose_api VERSION 0.1.0 LANGUAGES C CXX)

include(GNUInstallDirs)
include(CMakePackageConfigHelpers)

# Detect whether we are the top-level project (CMake < 3.21 has no
# PROJECT_IS_TOP_LEVEL).
if(CMAKE_SOURCE_DIR STREQUAL CMAKE_CURRENT_SOURCE_DIR)
    set(_enpose_is_top_level ON)
else()
    set(_enpose_is_top_level OFF)
endif()

option(ENPOSE_BUILD_EXAMPLES "Build the C and C++ examples" ${_enpose_is_top_level})
set(ENPOSE_CARGO_PROFILE "release" CACHE STRING "Cargo build profile: release or debug")

# ---------------------------------------------------------------------------
# Build the Rust shared library via cargo.
# ---------------------------------------------------------------------------
find_program(CARGO_EXECUTABLE cargo REQUIRED)

set(ENPOSE_RUST_DIR "${CMAKE_CURRENT_SOURCE_DIR}/rust")

if(ENPOSE_CARGO_PROFILE STREQUAL "debug")
    set(CARGO_BUILD_FLAGS "")
    set(CARGO_OUT_DIR "${ENPOSE_RUST_DIR}/target/debug")
else()
    set(CARGO_BUILD_FLAGS "--release")
    set(CARGO_OUT_DIR "${ENPOSE_RUST_DIR}/target/release")
endif()

# cargo names its output after the Rust *target* ABI, which is not necessarily
# the C toolchain's platform convention (CMAKE_SHARED_LIBRARY_*). On Windows the
# two diverge: a MinGW CMake's convention is a "lib" prefix, but cargo emits
# "enpose_api.dll" with no prefix, and the import-library extension differs
# between the -gnu (.dll.a) and -msvc (.dll.lib) targets. Derive the artifact
# names from the Rust host triple instead, so the imported library resolves for
# whichever toolchain is in use.
execute_process(
    COMMAND "${CARGO_EXECUTABLE}" -vV
    OUTPUT_VARIABLE _enpose_cargo_vv
    OUTPUT_STRIP_TRAILING_WHITESPACE)
string(REGEX MATCH "host: ([^\r\n]+)" _ "${_enpose_cargo_vv}")
set(ENPOSE_RUST_HOST "${CMAKE_MATCH_1}")

# ENPOSE_IMPLIB is the link-time import library; it exists only for Windows
# targets and is left empty elsewhere (Linux/macOS link the shared object).
set(ENPOSE_IMPLIB "")
if(ENPOSE_RUST_HOST MATCHES "windows")
    set(ENPOSE_LIB_NAME "enpose_api.dll")
    if(ENPOSE_RUST_HOST MATCHES "gnu")
        # *-windows-gnu: MinGW-style import library (untested path).
        set(ENPOSE_IMPLIB "${CARGO_OUT_DIR}/enpose_api.dll.a")
    else()
        # *-windows-msvc: cargo emits <name>.dll.lib next to the DLL.
        set(ENPOSE_IMPLIB "${CARGO_OUT_DIR}/enpose_api.dll.lib")
    endif()
elseif(ENPOSE_RUST_HOST MATCHES "darwin|apple")
    set(ENPOSE_LIB_NAME "libenpose_api.dylib")
else()
    set(ENPOSE_LIB_NAME "libenpose_api.so")
endif()
set(ENPOSE_LIB "${CARGO_OUT_DIR}/${ENPOSE_LIB_NAME}")

add_custom_command(
    OUTPUT "${ENPOSE_LIB}"
    COMMAND ${CARGO_EXECUTABLE} build ${CARGO_BUILD_FLAGS}
    WORKING_DIRECTORY "${ENPOSE_RUST_DIR}"
    COMMENT "Building enpose_api (${ENPOSE_CARGO_PROFILE}) via cargo"
    VERBATIM)

# At top level, build the library as part of the default build so install and
# package targets always have it. As a subproject (add_subdirectory /
# FetchContent), build it on demand when a consumer links a target.
if(_enpose_is_top_level)
    add_custom_target(enpose_api_cargo ALL DEPENDS "${ENPOSE_LIB}")
else()
    add_custom_target(enpose_api_cargo DEPENDS "${ENPOSE_LIB}")
endif()

# ---------------------------------------------------------------------------
# Targets for in-tree consumers (add_subdirectory / FetchContent).
# ---------------------------------------------------------------------------
# The compiled library, imported.
add_library(enpose_api_shared SHARED IMPORTED GLOBAL)
set_target_properties(enpose_api_shared PROPERTIES IMPORTED_LOCATION "${ENPOSE_LIB}")

# A Windows shared library is linked against its import library, not the DLL
# itself, so the imported target needs IMPORTED_IMPLIB there. ENPOSE_IMPLIB is
# empty on platforms that link the shared object directly (Linux/macOS), where
# this is skipped.
if(ENPOSE_IMPLIB)
    set_target_properties(enpose_api_shared PROPERTIES IMPORTED_IMPLIB "${ENPOSE_IMPLIB}")
endif()

# C interface: the shared library + the C header. The cargo build dependency
# lives on this (non-imported) target so it propagates to consumers that link it.
add_library(enpose_api INTERFACE)
add_dependencies(enpose_api enpose_api_cargo)
target_link_libraries(enpose_api INTERFACE enpose_api_shared)
target_include_directories(enpose_api INTERFACE
    "$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/c/include>"
    "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>")
add_library(enpose_api::enpose_api ALIAS enpose_api)

# C++ interface: header-only wrapper on top of the C interface.
add_library(enpose_api_cpp INTERFACE)
target_link_libraries(enpose_api_cpp INTERFACE enpose_api)
target_include_directories(enpose_api_cpp INTERFACE
    "$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/cpp/include>"
    "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>")
target_compile_features(enpose_api_cpp INTERFACE cxx_std_17)
add_library(enpose_api::enpose_api_cpp ALIAS enpose_api_cpp)

# ---------------------------------------------------------------------------
# Install rules + config package.
#
# Files are split across two install components:
#   * sdk    -- headers, the shared library, and the CMake package config. This
#               is the entire SDK; it is all a plain `cmake --install` puts in
#               place, and all a find_package consumer needs.
#   * extras -- the LICENSE, per-binding docs, and the buildable examples. These
#               are EXCLUDE_FROM_ALL, so they are left out of a plain
#               `cmake --install` and only bundled into the distributable
#               package (see the CPack section below).
# ---------------------------------------------------------------------------
install(FILES
    "${CMAKE_CURRENT_SOURCE_DIR}/c/include/enpose_api.h"
    "${CMAKE_CURRENT_SOURCE_DIR}/cpp/include/enpose_api.hpp"
    DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}"
    COMPONENT sdk)

install(FILES "${ENPOSE_LIB}"
    DESTINATION "${CMAKE_INSTALL_LIBDIR}"
    COMPONENT sdk)

# Windows consumers link the import library, so ship it alongside the DLL.
if(ENPOSE_IMPLIB)
    install(FILES "${ENPOSE_IMPLIB}"
        DESTINATION "${CMAKE_INSTALL_LIBDIR}"
        COMPONENT sdk)
endif()

set(ENPOSE_CONFIG_INSTALL_DIR "${CMAKE_INSTALL_LIBDIR}/cmake/enpose_api")

configure_package_config_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/enpose_apiConfig.cmake.in"
    "${CMAKE_CURRENT_BINARY_DIR}/enpose_apiConfig.cmake"
    INSTALL_DESTINATION "${ENPOSE_CONFIG_INSTALL_DIR}"
    PATH_VARS CMAKE_INSTALL_INCLUDEDIR CMAKE_INSTALL_LIBDIR)

write_basic_package_version_file(
    "${CMAKE_CURRENT_BINARY_DIR}/enpose_apiConfigVersion.cmake"
    VERSION "${PROJECT_VERSION}"
    COMPATIBILITY SameMajorVersion)

install(FILES
    "${CMAKE_CURRENT_BINARY_DIR}/enpose_apiConfig.cmake"
    "${CMAKE_CURRENT_BINARY_DIR}/enpose_apiConfigVersion.cmake"
    DESTINATION "${ENPOSE_CONFIG_INSTALL_DIR}"
    COMPONENT sdk)

# ---------------------------------------------------------------------------
# Extras for the distributable package only (component `extras`,
# EXCLUDE_FROM_ALL keeps them out of a plain `cmake --install`):
#
#   * LICENSE   -- at the package top level.
#   * README.md -- consumer-facing overview of the unpacked package (its source
#                  lives in docsrc/package/README.md), at the top level.
#   * docs/     -- the generated API documentation for all four bindings, one
#                  HTML site per subfolder (c, cpp, python, rust).
#   * examples/ -- buildable sources for all four bindings, one subdir each:
#       - c, cpp:  example source + a CMakeLists.txt that configures against the
#                  installed SDK via find_package, so they build with no Rust
#                  toolchain (see c/example/CMakeLists.txt).
#       - rust:    the full crate (the native binding and the source of truth
#                  for the C ABI), built from source with `cargo`.
#       - python:  a cffi binding that loads the prebuilt shared library at run
#                  time (no compiler needed); just the package and an example.
# ---------------------------------------------------------------------------
install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/LICENSE"
    DESTINATION "."
    COMPONENT extras EXCLUDE_FROM_ALL)

install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/docsrc/package/README.md"
    DESTINATION "."
    COMPONENT extras EXCLUDE_FROM_ALL)

# The generated API documentation, one HTML site per binding (docs/c, docs/cpp,
# docs/python, docs/rust). The packaging step does not honour add_dependencies
# on the install staging, so instead regenerate the docs here, as the first step
# of installing the `extras` component — i.e. whenever a package is built — by
# invoking the `docs` target.
# A plain `cmake --install` installs only the `sdk` component, so it skips this.
install(CODE "
    message(STATUS \"enpose_api: generating API documentation for the package\")
    execute_process(
        COMMAND \"${CMAKE_COMMAND}\" -P \"${CMAKE_CURRENT_BINARY_DIR}/build_docs.cmake\"
        RESULT_VARIABLE _enpose_docs_rc)
    # build_docs.cmake is best-effort: it skips any binding whose generator is
    # missing or cannot run here, so the package carries whatever docs were
    # produced rather than failing outright. A non-zero result means the script
    # itself errored (not a skipped language); warn but still build the package.
    if(NOT _enpose_docs_rc EQUAL 0)
        message(WARNING \"enpose_api: documentation generation reported an error; packaging whatever docs were produced\")
    endif()"
    COMPONENT extras EXCLUDE_FROM_ALL)

# Install each binding's generated docs, but only if it was actually produced.
# The `docs` target above skips any language whose generator is missing
# (Doxygen for c/cpp, sphinx-build for python), so the corresponding
# docs/<lang> directory may not exist. A plain install(DIRECTORY) on a missing
# source is fatal, which would make the package unbuildable without the full
# doc toolchain. Copy at install time and skip whatever was not generated, so
# the package carries whatever docs are available. ${CMAKE_CURRENT_SOURCE_DIR}
# is baked in at configure time; the per-language checks run at install time.
install(CODE "
    foreach(_lang c cpp python rust)
        set(_docsrc \"${CMAKE_CURRENT_SOURCE_DIR}/docs/\${_lang}\")
        file(GLOB _doc_contents \"\${_docsrc}/*\")
        if(IS_DIRECTORY \"\${_docsrc}\" AND _doc_contents)
            file(INSTALL \"\${_docsrc}\"
                DESTINATION \"\${CMAKE_INSTALL_PREFIX}/docs\"
                USE_SOURCE_PERMISSIONS)
        else()
            message(STATUS \"enpose_api: [skip] docs/\${_lang} — not generated\")
        endif()
    endforeach()"
    COMPONENT extras EXCLUDE_FROM_ALL)

# C and C++: example source + a standalone CMakeLists that builds against the
# installed SDK via find_package.
install(FILES
    "${CMAKE_CURRENT_SOURCE_DIR}/c/example/example.c"
    "${CMAKE_CURRENT_SOURCE_DIR}/c/example/CMakeLists.txt"
    DESTINATION "examples/c"
    COMPONENT extras EXCLUDE_FROM_ALL)
install(FILES
    "${CMAKE_CURRENT_SOURCE_DIR}/cpp/example/example.cpp"
    "${CMAKE_CURRENT_SOURCE_DIR}/cpp/example/CMakeLists.txt"
    DESTINATION "examples/cpp"
    COMPONENT extras EXCLUDE_FROM_ALL)

# Rust: the full crate -- manifest, library sources (incl. the *_tests.rs files
# referenced via #[path]), and the example. Cargo.lock is intentionally omitted
# (it is untracked and absent on fresh checkouts; cargo regenerates it).
install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/rust/Cargo.toml"
    DESTINATION "examples/rust"
    COMPONENT extras EXCLUDE_FROM_ALL)
install(DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/rust/src"
    DESTINATION "examples/rust"
    COMPONENT extras EXCLUDE_FROM_ALL)
install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/rust/example/example.rs"
    DESTINATION "examples/rust/example"
    COMPONENT extras EXCLUDE_FROM_ALL)

# Python: the cffi binding package and its example.
install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/python/enpose_api/__init__.py"
    DESTINATION "examples/python/enpose_api"
    COMPONENT extras EXCLUDE_FROM_ALL)
install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/python/example/example.py"
    DESTINATION "examples/python/example"
    COMPONENT extras EXCLUDE_FROM_ALL)

# ---------------------------------------------------------------------------
# Distributable binary package (cmake --build build --target dist).
#
# A single archive bundling both components, so it carries the SDK (headers,
# library, CMake config) plus the extras: LICENSE at the top level, docs/, and
# examples/.
# ---------------------------------------------------------------------------
if(_enpose_is_top_level)
    set(CPACK_PACKAGE_NAME "enpose_api")
    set(CPACK_PACKAGE_VENDOR "Enpose")
    set(CPACK_PACKAGE_VERSION "${PROJECT_VERSION}")
    set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "Enpose 6-DoF tracking API (prebuilt SDK)")
    set(CPACK_RESOURCE_FILE_LICENSE "${CMAKE_CURRENT_SOURCE_DIR}/LICENSE")
    set(CPACK_GENERATOR "TGZ;ZIP")

    # Normalize the processor name so 64-bit x86 builds use "x64" on every
    # platform. CMAKE_SYSTEM_PROCESSOR reports "AMD64" on Windows and "x86_64"
    # on Linux for the same architecture.
    set(_enpose_package_arch "${CMAKE_SYSTEM_PROCESSOR}")
    if(_enpose_package_arch MATCHES "^(AMD64|amd64|x86_64|x64)$")
        set(_enpose_package_arch "x64")
    endif()
    set(CPACK_PACKAGE_FILE_NAME
        "enpose_api-${PROJECT_VERSION}-${CMAKE_SYSTEM_NAME}-${_enpose_package_arch}")

    # Merge every component -- including the EXCLUDE_FROM_ALL `extras` -- into a
    # single archive rather than emitting one archive per component.
    set(CPACK_ARCHIVE_COMPONENT_INSTALL ON)
    set(CPACK_COMPONENTS_GROUPING ALL_COMPONENTS_IN_ONE)
    set(CPACK_COMPONENTS_ALL sdk extras)

    # Component archives default to dumping their contents at the archive root.
    # Wrap them in a versioned enpose_api-<version>-<system>/ folder on Linux, but
    # unpack to the archive root on Windows -- both the MSVC and the MinGW builds,
    # which each define WIN32. In component mode this is the variable that takes
    # effect (it overrides CPACK_INCLUDE_TOPLEVEL_DIRECTORY).
    if(WIN32)
        set(CPACK_COMPONENT_INCLUDE_TOPLEVEL_DIRECTORY OFF)
    else()
        set(CPACK_COMPONENT_INCLUDE_TOPLEVEL_DIRECTORY ON)
    endif()

    include(CPack)

    # include(CPack) always adds a target literally named `package`; CMake offers
    # no way to rename it. Expose the build step under the project's preferred
    # name `dist`, which drives cpack against the generated CPackConfig.cmake and
    # runs the same component install (regenerating docs) that `package` would.
    add_custom_target(dist
        COMMAND "${CMAKE_CPACK_COMMAND}" --config "${CMAKE_CURRENT_BINARY_DIR}/CPackConfig.cmake"
        WORKING_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}"
        VERBATIM
        COMMENT "Building distributable package (TGZ + ZIP)")
    # cpack does not build the project first, so ensure the cargo-built shared
    # library exists before it is staged for the archive.
    add_dependencies(dist enpose_api_cargo)
endif()

# ---------------------------------------------------------------------------
# Examples.
# ---------------------------------------------------------------------------
if(ENPOSE_BUILD_EXAMPLES)
    add_subdirectory(c/example)
    add_subdirectory(cpp/example)
endif()

# ---------------------------------------------------------------------------
# API documentation (`docs` target).
#
# Generates per-binding API docs into <source>/docs/<language>/, each themed
# with the shared Enpose palette (sources under docsrc/; colours mirror
# webgui/src/my_ui.rs):
#   * python -> Sphinx  (furo theme + docsrc/python/_static/enpose.css)
#   * c, cpp -> Doxygen (HTML_EXTRA_STYLESHEET = docsrc/doxygen/enpose-doxygen.css)
#   * rust   -> rustdoc (cargo doc, --extend-css docsrc/rust/enpose-rustdoc.css)
#
# Run with:  cmake --build <build-dir> --target docs
# Each tool is located if present; a missing one skips that language with a note.
# ---------------------------------------------------------------------------
if(_enpose_is_top_level)
    set(ENPOSE_DOCS_DIR "${CMAKE_CURRENT_SOURCE_DIR}/docs")
    set(ENPOSE_DOCSRC_DIR "${CMAKE_CURRENT_SOURCE_DIR}/docsrc")

    # On Windows, prefer a real sphinx-build.exe over an extension-less wrapper
    # script: listing the .exe first makes find_program search every PATH dir for
    # it before falling back. Harmless on Linux (no .exe exists).
    find_program(SPHINX_BUILD_EXECUTABLE NAMES sphinx-build.exe sphinx-build)
    find_program(DOXYGEN_EXECUTABLE doxygen)
    # CARGO_EXECUTABLE and ENPOSE_RUST_DIR are set near the top of this file.

    # All doc generators are expected to come from the same toolchain as this
    # build (a native MSVC build uses native tools; an MSYS2/MinGW build uses the
    # mingw64/ucrt64 tools; Linux uses the system tools). Within one chain every
    # tool accepts the native paths CMake produces, so no path translation is
    # needed. Sphinx (Python) and doxygen are invoked directly in build_docs.cmake.

    # Doxygen is driven by one configured Doxyfile per language; configure them
    # here (at configure time) so build_docs.cmake can just invoke doxygen on the
    # generated files.
    if(DOXYGEN_EXECUTABLE)
        set(DOXY_EXTRA_CSS "${ENPOSE_DOCSRC_DIR}/doxygen/enpose-doxygen.css")

        set(DOXY_PROJECT_NAME "Enpose API — C")
        set(DOXY_MAINPAGE "${CMAKE_CURRENT_SOURCE_DIR}/c/README.md")
        set(DOXY_INPUT "${CMAKE_CURRENT_SOURCE_DIR}/c/include ${DOXY_MAINPAGE}")
        set(DOXY_OUTPUT "${ENPOSE_DOCS_DIR}/c")
        set(DOXY_FILE_PATTERNS "*.h *.md")
        set(DOXY_OPTIMIZE_C "YES")
        configure_file("${ENPOSE_DOCSRC_DIR}/doxygen/Doxyfile.in"
                       "${CMAKE_CURRENT_BINARY_DIR}/Doxyfile.c" @ONLY)

        set(DOXY_PROJECT_NAME "Enpose API — C++")
        set(DOXY_MAINPAGE "${CMAKE_CURRENT_SOURCE_DIR}/cpp/README.md")
        set(DOXY_INPUT "${CMAKE_CURRENT_SOURCE_DIR}/cpp/include ${DOXY_MAINPAGE}")
        set(DOXY_OUTPUT "${ENPOSE_DOCS_DIR}/cpp")
        set(DOXY_FILE_PATTERNS "*.hpp *.md")
        set(DOXY_OPTIMIZE_C "NO")
        configure_file("${ENPOSE_DOCSRC_DIR}/doxygen/Doxyfile.in"
                       "${CMAKE_CURRENT_BINARY_DIR}/Doxyfile.cpp" @ONLY)
    endif()

    # All per-language doc generation lives in build_docs.cmake, run via a single
    # `cmake -P`. Each language is generated independently and non-fatally there,
    # so a generator that is missing or cannot run in this environment is skipped
    # with a note instead of breaking the `docs` target or the `dist` package.
    # Using one CMake script also avoids per-generator shell quoting differences
    # between Make, Ninja and MSBuild.
    configure_file("${CMAKE_CURRENT_SOURCE_DIR}/cmake/build_docs.cmake.in"
                   "${CMAKE_CURRENT_BINARY_DIR}/build_docs.cmake" @ONLY)

    add_custom_target(docs
        COMMAND "${CMAKE_COMMAND}" -P "${CMAKE_CURRENT_BINARY_DIR}/build_docs.cmake"
        COMMENT "Generating API documentation into docs/<language>/"
        VERBATIM)
endif()
