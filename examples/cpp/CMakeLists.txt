# Copyright The Pit Project Owners. All rights reserved.
# SPDX-License-Identifier: Apache-2.0
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
# Please see https://openpit.dev and the OWNERS file for details.

# Top-level aggregator for the C++ examples. Configuring this directory builds
# all four examples (and their gtest smoke tests) against a single embedded copy
# of the OpenPit C++ binding, so `just test-examples-cpp` can drive them through
# one CMake configure + one ctest run. Each example also stays independently
# buildable on its own (configure its subdirectory directly); this file just
# wires the binding once and shares it across all of them.
#
# Provide the native runtime exactly as the binding documents, e.g.:
#   -DOPENPIT_RUNTIME_LIBRARY=$(pwd)/target/release/libopenpit_ffi.dylib
# Omit it to let the binding download the matching prebuilt runtime release.

cmake_minimum_required(VERSION 3.21)

if(POLICY CMP0091)
  cmake_policy(SET CMP0091 NEW)
endif()

project(openpit_examples_cpp
  DESCRIPTION "OpenPit C++ examples aggregator"
  LANGUAGES CXX)

if(MSVC AND NOT DEFINED CMAKE_MSVC_RUNTIME_LIBRARY)
  set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")
endif()

# Resolve the OpenPit C++ binding once, here, so every example picks up the same
# OpenPit::openpit target via the `TARGET OpenPit::openpit` shortcut each of them
# already checks. The binding records the resolved runtime path in
# OPENPIT_RUNTIME_LIBRARY_FILE, which the examples read to embed BUILD_RPATH.
get_filename_component(OPENPIT_CPP_BINDING_DIR
  "${CMAKE_CURRENT_SOURCE_DIR}/../../bindings/cpp" ABSOLUTE)

if(NOT TARGET OpenPit::openpit)
  if(EXISTS "${OPENPIT_CPP_BINDING_DIR}/CMakeLists.txt")
    # The binding's own tests are off when embedded; each example ships its own.
    set(OPENPIT_CPP_BUILD_TESTS OFF CACHE BOOL "" FORCE)
    add_subdirectory("${OPENPIT_CPP_BINDING_DIR}" openpit-cpp-binding)
  else()
    find_package(OpenPit CONFIG REQUIRED)
  endif()
endif()

# Build every example's smoke test, even though we add them as subdirectories
# (which leaves PROJECT_IS_TOP_LEVEL false for each). The examples gate their
# tests on these cache variables.
enable_testing()
set(OPENPIT_EXAMPLE_BUILD_TESTS ON CACHE BOOL "" FORCE)
set(SPOT_LOADTEST_BUILD_TESTS ON CACHE BOOL "" FORCE)

add_subdirectory(rate_pnl_killswitch)
add_subdirectory(spot_funds)
add_subdirectory(spot_table)
add_subdirectory(spot_loadtest)
