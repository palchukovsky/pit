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

#include <openpit/openpit.hpp>
#include <openpit/pretrade/policies.hpp>

#include <iostream>
#include <string>

int main() {
  const std::string version = openpit::GetVersion();
  if (version.empty()) {
    std::cerr << "OpenPit version is empty\n";
    return 1;
  }

  openpit::EngineBuilder builder(openpit::SyncPolicy::None);
  builder.Add(openpit::pretrade::policies::OrderValidationPolicy{});
  const openpit::Engine engine = builder.Build();
  if (!engine) {
    std::cerr << "OpenPit engine handle is empty\n";
    return 1;
  }

  std::cout << "OpenPit C++ consumer loaded " << version << "\n";
  return 0;
}
