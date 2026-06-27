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

#include <openpit.h>

#include <cstdint>
#include <string>

namespace openpit::marketdata {

// Type-safe market-data instrument identifier.
//
// A service assigns a stable id the first time an instrument is registered; all
// later reads and writes use that id rather than resolving the instrument by
// name. Trivial wrapper over the native runtime `uint64_t`.
class InstrumentId {
 public:
  constexpr InstrumentId() noexcept = default;

  explicit constexpr InstrumentId(OpenPitMarketDataInstrumentId value) noexcept
      : m_value(value) {}

  // Constructs an instrument id from a raw `uint64` value.
  [[nodiscard]] static constexpr InstrumentId FromUint64(
      std::uint64_t value) noexcept {
    return InstrumentId(value);
  }

  [[nodiscard]] constexpr OpenPitMarketDataInstrumentId Raw() const noexcept {
    return m_value;
  }

  // Decimal rendering of the underlying id.
  [[nodiscard]] std::string ToString() const { return std::to_string(m_value); }

  [[nodiscard]] constexpr bool operator==(
      const InstrumentId& other) const noexcept {
    return m_value == other.m_value;
  }
  [[nodiscard]] constexpr bool operator!=(
      const InstrumentId& other) const noexcept {
    return m_value != other.m_value;
  }

 private:
  OpenPitMarketDataInstrumentId m_value = 0;
};

}  // namespace openpit::marketdata
