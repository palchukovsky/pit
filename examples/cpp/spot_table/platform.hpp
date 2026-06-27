// Copyright The Pit Project Owners. All rights reserved.
// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// Please see https://openpit.dev and the OWNERS file for details.

#pragma once

// at the head of each run's report. Detection is best-effort: a field that
// cannot be determined stays "unknown" rather than failing the run. In place of
// OpenPit runtime version (the analogue of "which toolchain produced this
// run").

#include <string>

namespace spot_table {

struct PlatformInfo {
  std::string hardware = "unknown";
  std::string cpu = "unknown";
  int cores = 0;
  std::string memory = "unknown";
  std::string disk = "unknown";
  std::string os = "unknown";
  std::string arch = "unknown";
  std::string toolchain = "unknown";
};

[[nodiscard]] PlatformInfo GatherPlatform();

// `printPlatform`.
void PrintPlatform();

} // namespace spot_table
