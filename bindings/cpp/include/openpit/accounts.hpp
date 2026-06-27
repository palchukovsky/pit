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

#include "openpit/account_id.hpp"
#include "openpit/detail/handle.hpp"
#include "openpit/error.hpp"
#include "openpit/reject.hpp"
#include "openpit/string.hpp"

#include <openpit.h>

#include <cstdint>
#include <optional>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

// Account administration surface.
//
// `Accounts` is the engine.accounts() admin handle: it manages account-group
// membership and records or lifts pre-trade blocks, addressable both by
// individual `param::AccountId` and by an account-group predicate
// (`param::AccountGroupId`, i.e. membership in that group), over the one
// unified blocked-accounts list the engine owns. Account blocking is owned by
// the engine; this handle only forwards to it.
//
// Block/unblock by id are infallible. Reason-replacement and every group-scoped
// operation can fail with an expected, structured outcome - returned as a
// `std::optional` value, never thrown. runtime boundary failures (a C call
// writing its `out_error`) throw `openpit::Error`.
//
// `AccountControl` is the engine-provided handle a custom callback uses to
// record a kill-switch block against the account bound to its context; it is
// move-only RAII and valid only within the pre-trade transaction that produced
// it. `AccountBlock` is the value type that block carries.

namespace openpit::accounts {

// A kill-switch block record. Produced by policy callbacks and read back from
// `Engine::ApplyExecutionReport`; also the payload recorded through
// `AccountControl::Block`. `userData` is an opaque caller token the SDK never
// inspects (zero means unset).
//
// Field order follows the native runtime `OpenPitPretradeAccountBlock` (string
// views first, then the pointer and the code) so view conversion stays
// mechanical.
struct AccountBlock {
  std::string policy;
  std::string reason;
  std::string details;
  std::uintptr_t userData = 0;
  ::openpit::pretrade::RejectCode code = ::openpit::pretrade::RejectCode::Other;

  AccountBlock() = default;

  AccountBlock(::openpit::pretrade::RejectCode blockCode,
               std::string policyName, std::string blockReason,
               std::string blockDetails)
      : policy(std::move(policyName)),
        reason(std::move(blockReason)),
        details(std::move(blockDetails)),
        code(blockCode) {}

  // Copies the borrowed string views out of a C account-block record.
  [[nodiscard]] static AccountBlock FromRaw(
      const OpenPitPretradeAccountBlock& raw) {
    AccountBlock out;
    out.policy = ::openpit::StringView(raw.policy).ToString();
    out.reason = ::openpit::StringView(raw.reason).ToString();
    out.details = ::openpit::StringView(raw.details).ToString();
    out.userData = reinterpret_cast<std::uintptr_t>(raw.user_data);
    out.code = static_cast<::openpit::pretrade::RejectCode>(raw.code);
    return out;
  }

  // Builds a C account-block record whose string views borrow this object's
  // strings; valid only while this `AccountBlock` is alive and unchanged.
  [[nodiscard]] OpenPitPretradeAccountBlock Raw() const noexcept {
    OpenPitPretradeAccountBlock raw{};
    raw.policy = ::openpit::MakeStringView(policy);
    raw.reason = ::openpit::MakeStringView(reason);
    raw.details = ::openpit::MakeStringView(details);
    raw.user_data = reinterpret_cast<void*>(userData);
    raw.code = static_cast<OpenPitPretradeRejectCode>(
        static_cast<std::uint16_t>(code));
    return raw;
  }
};

namespace detail {

struct AccountControlDeleter {
  void operator()(OpenPitAccountControl* handle) const noexcept {
    openpit_destroy_account_control(handle);
  }
};

}  // namespace detail

// Engine-provided handle that records kill-switch blocks against the account
// bound to a callback context.
//
// Move-only RAII; destruction releases the handle. It is valid to use only
// within the pre-trade transaction of the request it belongs to - from the
// callback that produced it through the commit or rollback of that request's
// reservation, so it may be captured for deferred blocking. Recording a block
// through it afterwards is undefined.
class AccountControl {
 public:
  AccountControl() = default;

  // Adopts a caller-owned handle returned by the native runtime.
  explicit AccountControl(OpenPitAccountControl* handle) noexcept
      : m_handle(handle) {}

  [[nodiscard]] explicit operator bool() const noexcept {
    return static_cast<bool>(m_handle);
  }

  // Records `block` against the bound account. The first cause recorded for an
  // account wins; later calls for the same account are no-ops.
  void Block(const AccountBlock& block) const noexcept {
    openpit_account_control_block(m_handle.Get(), block.Raw());
  }

  // Returns a new handle referring to the same account-control facility, for
  // retaining the ability to block from a later callback within the same
  // pre-trade transaction. Throws `openpit::Error` when this handle is empty.
  [[nodiscard]] AccountControl Clone() const {
    OpenPitAccountControl* raw = openpit_account_control_clone(m_handle.Get());
    if (raw == nullptr) {
      throw ::openpit::Error("openpit_account_control_clone failed");
    }
    return AccountControl(raw);
  }

  [[nodiscard]] OpenPitAccountControl* Get() const noexcept {
    return m_handle.Get();
  }

 private:
  ::openpit::detail::Handle<OpenPitAccountControl,
                            detail::AccountControlDeleter>
      m_handle;
};

// Classifies an `AccountBlockError`. Mirrors `OpenPitAccountBlockErrorKind`.
enum class AccountBlockErrorKind : std::uint32_t {
  // The targeted group is the reserved `param::DefaultAccountGroup`, which
  // cannot be blocked, unblocked, or have its reason replaced.
  ReservedGroup = OpenPitAccountBlockErrorKind_ReservedGroup,
  // A reason replacement targeted an account that is not blocked.
  AccountNotBlocked = OpenPitAccountBlockErrorKind_AccountNotBlocked,
  // A group operation targeted a group that is not blocked.
  GroupNotBlocked = OpenPitAccountBlockErrorKind_GroupNotBlocked,
};

// Expected outcome of a failed account- or group-block operation. A value type,
// never thrown. `Block`/`Unblock` are infallible; only `ReplaceBlockReason`,
// `BlockGroup`, `UnblockGroup`, and `ReplaceGroupBlockReason` can produce one.
// `Account` is set only for `AccountNotBlocked`, `Group` only for
// `GroupNotBlocked`.
struct AccountBlockError {
  std::string message;
  std::optional<::openpit::param::AccountId> account;
  std::optional<::openpit::param::AccountGroupId> group;
  AccountBlockErrorKind kind = AccountBlockErrorKind::ReservedGroup;

  // Copies fields out of a caller-owned C error and releases it.
  [[nodiscard]] static AccountBlockError FromHandle(
      OpenPitAccountBlockError* handle) {
    AccountBlockError out;
    out.message =
        ::openpit::StringView(openpit_account_block_error_get_message(handle))
            .ToString();
    out.kind = static_cast<AccountBlockErrorKind>(
        openpit_account_block_error_get_kind(handle));
    OpenPitParamAccountId accountId = 0;
    if (openpit_account_block_error_get_account(handle, &accountId)) {
      out.account = ::openpit::param::AccountId::FromRaw(accountId);
    }
    OpenPitParamAccountGroupId groupId = 0;
    if (openpit_account_block_error_get_group(handle, &groupId)) {
      out.group = ::openpit::param::AccountGroupId::FromRaw(groupId);
    }
    openpit_destroy_account_block_error(handle);
    return out;
  }
};

// Expected outcome of a failed `RegisterGroup` / `UnregisterGroup`: a group
// conflict or a reserved default-group target. A value type, never thrown.
// `currentGroup` is set when the conflict is a duplicate registration (the
// group the account already belongs to) and absent for an unregister miss.
struct AccountGroupError {
  std::string message;
  ::openpit::param::AccountId account;
  std::optional<::openpit::param::AccountGroupId> currentGroup;

  // Copies fields out of a caller-owned C error and releases it.
  [[nodiscard]] static AccountGroupError FromHandle(
      OpenPitAccountGroupError* handle) {
    AccountGroupError out;
    out.message =
        ::openpit::StringView(openpit_account_group_error_get_message(handle))
            .ToString();
    out.account = ::openpit::param::AccountId::FromRaw(
        openpit_account_group_error_get_account(handle));
    OpenPitParamAccountGroupId groupId = 0;
    if (openpit_account_group_error_get_current_group(handle, &groupId)) {
      out.currentGroup = ::openpit::param::AccountGroupId::FromRaw(groupId);
    }
    openpit_destroy_account_group_error(handle);
    return out;
  }
};

/// \brief Account-group and account/group block administration for an engine.
//
// Account-group management and account/group pre-trade blocking bound to an
// engine. Obtained from `Engine::Accounts()`. It carries no state of its own:
// every call forwards to the engine it was created from and is valid for as
// long as that engine is. Non-owning; copyable.
class Accounts {
 public:
  Accounts() = default;

  // Wraps a borrowed engine handle. The engine retains ownership.
  explicit Accounts(OpenPitEngine* engine) noexcept : m_engine(engine) {}

  // Atomically registers every account into `group`; all-or-nothing. Returns an
  // `AccountGroupError` when any account is already in a group or when `group`
  // is the reserved `param::DefaultAccountGroup`. Throws `openpit::Error` on a
  // boundary failure.
  [[nodiscard]] std::optional<AccountGroupError> RegisterGroup(
      const std::vector<::openpit::param::AccountId>& accounts,
      ::openpit::param::AccountGroupId group) const {
    return GroupOp(openpit_engine_register_account_group, accounts, group,
                   "openpit_engine_register_account_group failed");
  }

  // Atomically removes every account from `group`; all-or-nothing. Returns an
  // `AccountGroupError` when any account is not in `group` or when `group` is
  // the reserved `param::DefaultAccountGroup`. Throws `openpit::Error` on a
  // boundary failure.
  [[nodiscard]] std::optional<AccountGroupError> UnregisterGroup(
      const std::vector<::openpit::param::AccountId>& accounts,
      ::openpit::param::AccountGroupId group) const {
    return GroupOp(openpit_engine_unregister_account_group, accounts, group,
                   "openpit_engine_unregister_account_group failed");
  }

  // The account-group of `account`, or `std::nullopt` when it belongs to none.
  [[nodiscard]] std::optional<::openpit::param::AccountGroupId> GroupOf(
      ::openpit::param::AccountId account) const {
    OpenPitParamAccountGroupId group = 0;
    if (openpit_engine_account_group(m_engine, account.Raw(), &group)) {
      return ::openpit::param::AccountGroupId::FromRaw(group);
    }
    return std::nullopt;
  }

  // Blocks `account` with `reason`, gating its pre-trade orders until
  // unblocked. The first reason recorded for an account wins; `reason` may be
  // empty.
  void Block(::openpit::param::AccountId account,
             std::string_view reason) const noexcept {
    openpit_engine_block_account(m_engine, account.Raw(),
                                 ::openpit::MakeStringView(reason));
  }

  // Lifts the block on `account`. Unblocking an unblocked account is a no-op.
  void Unblock(::openpit::param::AccountId account) const noexcept {
    openpit_engine_unblock_account(m_engine, account.Raw());
  }

  // Replaces the recorded reason of a blocked account. Returns an
  // `AccountBlockError` with kind `AccountNotBlocked` when `account` is not
  // blocked.
  [[nodiscard]] std::optional<AccountBlockError> ReplaceBlockReason(
      ::openpit::param::AccountId account, std::string_view reason) const {
    OpenPitAccountBlockError* error = nullptr;
    openpit_engine_replace_account_block_reason(
        m_engine, account.Raw(), ::openpit::MakeStringView(reason), &error);
    if (error != nullptr) {
      return AccountBlockError::FromHandle(error);
    }
    return std::nullopt;
  }

  // Blocks `group` with `reason`, gating the pre-trade orders of every account
  // in it. The first reason recorded for a group wins; `reason` may be empty.
  // Returns an `AccountBlockError` with kind `ReservedGroup` when `group` is
  // the reserved `param::DefaultAccountGroup`.
  [[nodiscard]] std::optional<AccountBlockError> BlockGroup(
      ::openpit::param::AccountGroupId group, std::string_view reason) const {
    OpenPitAccountBlockError* error = nullptr;
    openpit_engine_block_account_group(
        m_engine, group.Raw(), ::openpit::MakeStringView(reason), &error);
    if (error != nullptr) {
      return AccountBlockError::FromHandle(error);
    }
    return std::nullopt;
  }

  // Lifts the block on `group`; accounts blocked individually stay blocked.
  // Unblocking an unblocked group is a no-op. Returns an `AccountBlockError`
  // with kind `ReservedGroup` when `group` is the reserved
  // `param::DefaultAccountGroup`.
  [[nodiscard]] std::optional<AccountBlockError> UnblockGroup(
      ::openpit::param::AccountGroupId group) const {
    OpenPitAccountBlockError* error = nullptr;
    openpit_engine_unblock_account_group(m_engine, group.Raw(), &error);
    if (error != nullptr) {
      return AccountBlockError::FromHandle(error);
    }
    return std::nullopt;
  }

  // Replaces the recorded reason of a blocked group. Returns an
  // `AccountBlockError` with kind `ReservedGroup` when `group` is the reserved
  // `param::DefaultAccountGroup`, or `GroupNotBlocked` when `group` is not
  // blocked.
  [[nodiscard]] std::optional<AccountBlockError> ReplaceGroupBlockReason(
      ::openpit::param::AccountGroupId group, std::string_view reason) const {
    OpenPitAccountBlockError* error = nullptr;
    openpit_engine_replace_account_group_block_reason(
        m_engine, group.Raw(), ::openpit::MakeStringView(reason), &error);
    if (error != nullptr) {
      return AccountBlockError::FromHandle(error);
    }
    return std::nullopt;
  }

 private:
  // Shared register/unregister body. `fn` is the matching native runtime
  // symbol; both have an identical signature.
  template <typename Fn>
  [[nodiscard]] std::optional<AccountGroupError> GroupOp(
      Fn fn, const std::vector<::openpit::param::AccountId>& accounts,
      ::openpit::param::AccountGroupId group, const char* fallback) const {
    std::vector<OpenPitParamAccountId> raw;
    raw.reserve(accounts.size());
    for (const auto& account : accounts) {
      raw.push_back(account.Raw());
    }
    OpenPitAccountGroupError* groupError = nullptr;
    OpenPitSharedString* error = nullptr;
    const bool ok = fn(m_engine, raw.empty() ? nullptr : raw.data(), raw.size(),
                       group.Raw(), &groupError, &error);
    if (ok) {
      return std::nullopt;
    }
    if (groupError != nullptr) {
      return AccountGroupError::FromHandle(groupError);
    }
    ::openpit::detail::ThrowFromSharedString(error, fallback);
  }

  OpenPitEngine* m_engine = nullptr;
};

}  // namespace openpit::accounts
