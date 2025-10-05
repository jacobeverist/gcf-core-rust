# ==============================================================================
# CMakeLists.txt
# ==============================================================================
cmake_minimum_required(VERSION 3.7)
project(GNOMICS)

# Default flags
set(CMAKE_CXX_STANDARD 11)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Disable fopen secure warnings
if(MSVC)
    add_definitions(-D_CRT_SECURE_NO_WARNINGS)
endif()

# Build bbcore
add_subdirectory(src/cpp)

# If called then build C++ tests
if (GNOMICS_TESTS)
    add_subdirectory(tests/cpp)
endif()
