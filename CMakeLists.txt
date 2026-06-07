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
#     (cmake --build build --target package) and find_package(enpose_api CONFIG)
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

set(ENPOSE_LIB_NAME "${CMAKE_SHARED_LIBRARY_PREFIX}enpose_api${CMAKE_SHARED_LIBRARY_SUFFIX}")
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
# Install rules + config package (used by the binary package and by
# `cmake --install` consumers via find_package).
# ---------------------------------------------------------------------------
install(FILES
    "${CMAKE_CURRENT_SOURCE_DIR}/c/include/enpose_api.h"
    "${CMAKE_CURRENT_SOURCE_DIR}/cpp/include/enpose_api.hpp"
    DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}")

install(FILES "${ENPOSE_LIB}" DESTINATION "${CMAKE_INSTALL_LIBDIR}")

install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/LICENSE"
    DESTINATION "${CMAKE_INSTALL_DOCDIR}")

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
    DESTINATION "${ENPOSE_CONFIG_INSTALL_DIR}")

# Bundle the buildable sources for all three interfaces under examples/, each
# with its per-interface README, mirroring the source layout:
#   * C and C++: example sources + a CMakeLists.txt that configures against the
#     installed SDK via find_package, so they build with no Rust toolchain (see
#     c/example/CMakeLists.txt).
#   * Rust: the full crate (the native binding and the source of truth for the C
#     ABI), built from source with `cargo` like any standalone crate.
#   * Python: a cffi binding that loads the prebuilt shared library at run time
#     (no compiler needed); just the package, an example, and a README.
set(ENPOSE_EXAMPLES_INSTALL_DIR "${CMAKE_INSTALL_DOCDIR}/examples")

install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/c/README.md"
    DESTINATION "${ENPOSE_EXAMPLES_INSTALL_DIR}/c")
install(FILES
    "${CMAKE_CURRENT_SOURCE_DIR}/c/example/example.c"
    "${CMAKE_CURRENT_SOURCE_DIR}/c/example/CMakeLists.txt"
    DESTINATION "${ENPOSE_EXAMPLES_INSTALL_DIR}/c/example")

install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/cpp/README.md"
    DESTINATION "${ENPOSE_EXAMPLES_INSTALL_DIR}/cpp")
install(FILES
    "${CMAKE_CURRENT_SOURCE_DIR}/cpp/example/example.cpp"
    "${CMAKE_CURRENT_SOURCE_DIR}/cpp/example/CMakeLists.txt"
    DESTINATION "${ENPOSE_EXAMPLES_INSTALL_DIR}/cpp/example")

# The full Rust crate: manifest, library sources (incl. the *_tests.rs files
# referenced via #[path]), and the example. Cargo.lock is intentionally omitted
# (it is untracked and absent on fresh checkouts; cargo regenerates it).
install(FILES
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/README.md"
    "${CMAKE_CURRENT_SOURCE_DIR}/rust/Cargo.toml"
    DESTINATION "${ENPOSE_EXAMPLES_INSTALL_DIR}/rust")
install(DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/rust/src"
    DESTINATION "${ENPOSE_EXAMPLES_INSTALL_DIR}/rust")
install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/rust/example/example.rs"
    DESTINATION "${ENPOSE_EXAMPLES_INSTALL_DIR}/rust/example")

# The Python cffi binding: the package, its example, and the README.
install(FILES
    "${CMAKE_CURRENT_SOURCE_DIR}/python/README.md"
    DESTINATION "${ENPOSE_EXAMPLES_INSTALL_DIR}/python")
install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/python/enpose_api/__init__.py"
    DESTINATION "${ENPOSE_EXAMPLES_INSTALL_DIR}/python/enpose_api")
install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/python/example/example.py"
    DESTINATION "${ENPOSE_EXAMPLES_INSTALL_DIR}/python/example")

# ---------------------------------------------------------------------------
# Binary package (cmake --build build --target package).
# ---------------------------------------------------------------------------
if(_enpose_is_top_level)
    set(CPACK_PACKAGE_NAME "enpose_api")
    set(CPACK_PACKAGE_VENDOR "Enpose")
    set(CPACK_PACKAGE_VERSION "${PROJECT_VERSION}")
    set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "Enpose 6-DoF tracking API (prebuilt SDK)")
    set(CPACK_RESOURCE_FILE_LICENSE "${CMAKE_CURRENT_SOURCE_DIR}/LICENSE")
    set(CPACK_GENERATOR "TGZ;ZIP")
    set(CPACK_PACKAGE_FILE_NAME
        "enpose_api-${PROJECT_VERSION}-${CMAKE_SYSTEM_NAME}-${CMAKE_SYSTEM_PROCESSOR}")
    include(CPack)
endif()

# ---------------------------------------------------------------------------
# Examples.
# ---------------------------------------------------------------------------
if(ENPOSE_BUILD_EXAMPLES)
    add_subdirectory(c/example)
    add_subdirectory(cpp/example)
endif()
