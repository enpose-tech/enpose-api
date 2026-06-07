# CMake build for the Enpose API C example.
#
# Configures in three contexts, picked automatically:
#   * as part of the top-level enpose-api project (-DENPOSE_BUILD_EXAMPLES=ON)
#   * standalone inside a source checkout      -> builds the in-tree library
#   * standalone against an installed/prebuilt SDK -> uses find_package
#
# Standalone (the prefix path is only needed for a prebuilt SDK):
#   cmake -S . -B build [-DCMAKE_PREFIX_PATH=<path-to-sdk>]
#   cmake --build build
#   ./build/enpose_example

cmake_minimum_required(VERSION 3.19)
project(enpose_api_example C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

# Resolve the enpose_api targets:
#   * already defined        -> part of the top-level project, nothing to do
#   * source tree two levels up -> add_subdirectory the in-tree project
#   * otherwise              -> an installed/prebuilt SDK, found via find_package
if(NOT TARGET enpose_api::enpose_api)
    if(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/../../CMakeLists.txt")
        set(ENPOSE_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
        add_subdirectory("${CMAKE_CURRENT_SOURCE_DIR}/../.." enpose_api_root)
    else()
        find_package(enpose_api CONFIG REQUIRED)
    endif()
endif()

add_executable(enpose_example example.c)
target_link_libraries(enpose_example PRIVATE enpose_api::enpose_api)
