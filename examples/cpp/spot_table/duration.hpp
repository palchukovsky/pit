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

// (`time.Duration.String`, `time.Duration.Round`, `time.ParseDuration`); this
// small module reproduces them for the parts the CLI uses (`-timeout`,
// `-min-duration`, and the report's latency / wall-clock fields).

#include <chrono>
#include <string>

namespace spot_table {

// "1m30s", "1h2m3s", with sub-second units (ns/µs/ms) for short durations.
[[nodiscard]] std::string FormatDuration(std::chrono::nanoseconds d);

// `time.Duration.Round`. A non-positive `unit` returns `d` unchanged.
[[nodiscard]] std::chrono::nanoseconds
RoundDuration(std::chrono::nanoseconds d, std::chrono::nanoseconds unit);

// Returns false and writes `err` on a malformed string. Mirrors
// `time.ParseDuration` for the unit set the CLI accepts (ns, us/µs, ms, s, m,
// h).
[[nodiscard]] bool ParseDuration(const std::string &text,
                                 std::chrono::nanoseconds &out,
                                 std::string &err);

} // namespace spot_table
