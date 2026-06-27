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

#include "openpit/error.hpp"
#include "openpit/string.hpp"

#include <openpit.h>

#include <cstdint>
#include <string>
#include <string_view>

// Identifier value types.
//
// `GroupId` is the policy-group tag (`uint16`) the engine embeds in outcomes;
// the reserved default is `0`. `AccountId` is the trading-account tag
// (`uint64`); `AccountGroupId` is the account-group tag (`uint32`). Their
// constructors cross the C boundary so the value stays identical across
// bindings. Construction failures (reserved `0`, empty string) are boundary
// failures and throw `openpit::Error`.

namespace openpit::param {

// Trading-account identifier. Trivial wrapper over the native runtime
// `uint64_t`.
//
// WARNING: use exactly one source model per runtime - either purely numeric
// ids (`FromUint64`) or purely string-derived ids (`FromString`). Mixing the
// two can collapse two distinct accounts onto one hashed key.
class AccountId {
 public:
  constexpr AccountId() noexcept = default;

  // Direct numeric mapping with no collision risk; always succeeds. The native
  // runtime carries the account id as a bare `uint64_t`, so the value is the id
  // (the same identity mapping the core's numeric constructor performs); store
  // it directly without crossing the boundary.
  [[nodiscard]] static AccountId FromUint64(std::uint64_t value) noexcept {
    AccountId out;
    out.m_value = value;
    return out;
  }

  // FNV-1a hash of the UTF-8 bytes; any non-empty string yields a stable id.
  // Throws openpit::Error on empty input. Distinct strings may collide; keep an
  // explicit mapping and use `FromUint64` when that is unacceptable.
  [[nodiscard]] static AccountId FromString(std::string_view value) {
    AccountId out;
    OpenPitParamError* error = nullptr;
    if (!openpit_create_param_account_id_from_string(
            ::openpit::MakeStringView(value), &out.m_value, &error)) {
      ::openpit::detail::ThrowFromParamError(
          error, "openpit_create_param_account_id_from_string failed");
    }
    return out;
  }

  // Adopts a value read from an engine payload.
  [[nodiscard]] static AccountId FromRaw(OpenPitParamAccountId raw) noexcept {
    AccountId out;
    out.m_value = raw;
    return out;
  }

  [[nodiscard]] constexpr OpenPitParamAccountId Raw() const noexcept {
    return m_value;
  }

  // Decimal rendering of the underlying id; always succeeds.
  [[nodiscard]] std::string ToString() const {
    OpenPitSharedString* handle = openpit_param_account_id_to_string(m_value);
    std::string result = ::openpit::SharedStringView(handle).ToString();
    openpit_destroy_shared_string(handle);
    return result;
  }

  [[nodiscard]] constexpr bool operator==(
      const AccountId& other) const noexcept {
    return m_value == other.m_value;
  }
  [[nodiscard]] constexpr bool operator!=(
      const AccountId& other) const noexcept {
    return m_value != other.m_value;
  }

 private:
  OpenPitParamAccountId m_value = 0;
};

// Policy-group identifier; tags a policy instance to a logical group. The
// reserved value `0` (`DefaultPolicyGroupId`) is assigned when a caller does
// not pick a group. Trivial wrapper over the native runtime `uint16_t`.
class GroupId {
 public:
  constexpr GroupId() noexcept = default;

  explicit constexpr GroupId(std::uint16_t value) noexcept : m_value(value) {}

  // The native runtime carries the policy group id as a bare `uint16_t`.
  [[nodiscard]] constexpr std::uint16_t Raw() const noexcept { return m_value; }

  [[nodiscard]] constexpr bool operator==(const GroupId& other) const noexcept {
    return m_value == other.m_value;
  }
  [[nodiscard]] constexpr bool operator!=(const GroupId& other) const noexcept {
    return m_value != other.m_value;
  }

 private:
  std::uint16_t m_value = OPENPIT_DEFAULT_POLICY_GROUP_ID;
};

// The default policy-group id used when a caller does not assign a group.
inline constexpr std::uint16_t DefaultPolicyGroupId =
    OPENPIT_DEFAULT_POLICY_GROUP_ID;

// Account-group identifier.
//
// WARNING: use exactly one source model per runtime - either purely numeric
// ids (`FromUint32`) or purely string-derived ids (`FromString`). Mixing the
// two can collapse two distinct groups onto one hashed key.
class AccountGroupId {
 public:
  // Adopts the reserved default account group (`0`); every account belongs to
  // it until registered into another.
  constexpr AccountGroupId() noexcept = default;

  // Direct numeric mapping with no collision risk. Throws openpit::Error when
  // `value` is the reserved default (`0`); use the default constructor for it.
  [[nodiscard]] static AccountGroupId FromUint32(std::uint32_t value) {
    AccountGroupId out;
    OpenPitSharedString* error = nullptr;
    if (!openpit_create_param_account_group_id_from_uint32(value, &out.m_value,
                                                           &error)) {
      ::openpit::detail::ThrowFromSharedString(
          error, "openpit_create_param_account_group_id_from_uint32 failed");
    }
    return out;
  }

  // FNV-1a 32-bit hash of the UTF-8 bytes; any non-empty string yields a
  // stable id. Throws openpit::Error on empty input. Distinct strings may
  // collide; keep an explicit mapping and use `FromUint32` when that is
  // unacceptable.
  [[nodiscard]] static AccountGroupId FromString(std::string_view value) {
    AccountGroupId out;
    OpenPitSharedString* error = nullptr;
    if (!openpit_create_param_account_group_id_from_string(
            ::openpit::MakeStringView(value), &out.m_value, &error)) {
      ::openpit::detail::ThrowFromSharedString(
          error, "openpit_create_param_account_group_id_from_string failed");
    }
    return out;
  }

  // Adopts a value read from an engine payload.
  [[nodiscard]] static AccountGroupId FromRaw(
      OpenPitParamAccountGroupId raw) noexcept {
    AccountGroupId out;
    out.m_value = raw;
    return out;
  }

  [[nodiscard]] constexpr OpenPitParamAccountGroupId Raw() const noexcept {
    return m_value;
  }

  [[nodiscard]] constexpr bool IsDefault() const noexcept {
    return m_value == OPENPIT_DEFAULT_ACCOUNT_GROUP;
  }

  // Decimal rendering of the underlying id.
  [[nodiscard]] std::string ToString() const { return std::to_string(m_value); }

  [[nodiscard]] constexpr bool operator==(
      const AccountGroupId& other) const noexcept {
    return m_value == other.m_value;
  }
  [[nodiscard]] constexpr bool operator!=(
      const AccountGroupId& other) const noexcept {
    return m_value != other.m_value;
  }

 private:
  OpenPitParamAccountGroupId m_value = OPENPIT_DEFAULT_ACCOUNT_GROUP;
};

// The reserved account group every account belongs to until registered into
// another; no constructor may produce it.
inline constexpr AccountGroupId DefaultAccountGroup{};

}  // namespace openpit::param
