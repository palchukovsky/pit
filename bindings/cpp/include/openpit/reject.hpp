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

#include "openpit/string.hpp"

#include <openpit.h>

#include <cstdint>
#include <string>
#include <utility>
#include <vector>

// Reject value types.
//
// A `Reject` is an expected business outcome of a pre-trade check, not an
// error, so it is a value type and never thrown. `RejectScope` / `RejectCode`
// map 1:1 from the native runtime enums.
//
// The canonical definitions live in `openpit::pretrade`, matching the public
// policy API. They are also re-exported under `openpit::reject` as a short
// namespace; both spellings name the same types.

namespace openpit::pretrade {

// Broad area to which a reject applies. Mirrors `OpenPitPretradeRejectScope`;
// zero is not a valid scope.
//
enum class RejectScope : std::uint8_t {
  Order = OpenPitPretradeRejectScope_Order,
  Account = OpenPitPretradeRejectScope_Account,
};

// Stable machine-readable reject classification. Mirrors
// `OpenPitPretradeRejectCode`. Unknown incoming codes map to `Other`.
//
enum class RejectCode : std::uint16_t {
  MissingRequiredField = OpenPitPretradeRejectCode_MissingRequiredField,
  InvalidFieldFormat = OpenPitPretradeRejectCode_InvalidFieldFormat,
  InvalidFieldValue = OpenPitPretradeRejectCode_InvalidFieldValue,
  UnsupportedOrderType = OpenPitPretradeRejectCode_UnsupportedOrderType,
  UnsupportedTimeInForce = OpenPitPretradeRejectCode_UnsupportedTimeInForce,
  UnsupportedOrderAttribute =
      OpenPitPretradeRejectCode_UnsupportedOrderAttribute,
  DuplicateClientOrderId = OpenPitPretradeRejectCode_DuplicateClientOrderId,
  TooLateToEnter = OpenPitPretradeRejectCode_TooLateToEnter,
  ExchangeClosed = OpenPitPretradeRejectCode_ExchangeClosed,
  UnknownInstrument = OpenPitPretradeRejectCode_UnknownInstrument,
  UnknownAccount = OpenPitPretradeRejectCode_UnknownAccount,
  UnknownVenue = OpenPitPretradeRejectCode_UnknownVenue,
  UnknownClearingAccount = OpenPitPretradeRejectCode_UnknownClearingAccount,
  UnknownCollateralAsset = OpenPitPretradeRejectCode_UnknownCollateralAsset,
  InsufficientFunds = OpenPitPretradeRejectCode_InsufficientFunds,
  InsufficientMargin = OpenPitPretradeRejectCode_InsufficientMargin,
  InsufficientPosition = OpenPitPretradeRejectCode_InsufficientPosition,
  CreditLimitExceeded = OpenPitPretradeRejectCode_CreditLimitExceeded,
  RiskLimitExceeded = OpenPitPretradeRejectCode_RiskLimitExceeded,
  OrderExceedsLimit = OpenPitPretradeRejectCode_OrderExceedsLimit,
  OrderQtyExceedsLimit = OpenPitPretradeRejectCode_OrderQtyExceedsLimit,
  OrderNotionalExceedsLimit =
      OpenPitPretradeRejectCode_OrderNotionalExceedsLimit,
  PositionLimitExceeded = OpenPitPretradeRejectCode_PositionLimitExceeded,
  ConcentrationLimitExceeded =
      OpenPitPretradeRejectCode_ConcentrationLimitExceeded,
  LeverageLimitExceeded = OpenPitPretradeRejectCode_LeverageLimitExceeded,
  RateLimitExceeded = OpenPitPretradeRejectCode_RateLimitExceeded,
  PnlKillSwitchTriggered = OpenPitPretradeRejectCode_PnlKillSwitchTriggered,
  AccountBlocked = OpenPitPretradeRejectCode_AccountBlocked,
  AccountNotAuthorized = OpenPitPretradeRejectCode_AccountNotAuthorized,
  ComplianceRestriction = OpenPitPretradeRejectCode_ComplianceRestriction,
  InstrumentRestricted = OpenPitPretradeRejectCode_InstrumentRestricted,
  JurisdictionRestriction = OpenPitPretradeRejectCode_JurisdictionRestriction,
  WashTradePrevention = OpenPitPretradeRejectCode_WashTradePrevention,
  SelfMatchPrevention = OpenPitPretradeRejectCode_SelfMatchPrevention,
  ShortSaleRestriction = OpenPitPretradeRejectCode_ShortSaleRestriction,
  RiskConfigurationMissing = OpenPitPretradeRejectCode_RiskConfigurationMissing,
  ReferenceDataUnavailable = OpenPitPretradeRejectCode_ReferenceDataUnavailable,
  OrderValueCalculationFailed =
      OpenPitPretradeRejectCode_OrderValueCalculationFailed,
  SystemUnavailable = OpenPitPretradeRejectCode_SystemUnavailable,
  MarkPriceUnavailable = OpenPitPretradeRejectCode_MarkPriceUnavailable,
  AccountAdjustmentBoundsExceeded =
      OpenPitPretradeRejectCode_AccountAdjustmentBoundsExceeded,
  ArithmeticOverflow = OpenPitPretradeRejectCode_ArithmeticOverflow,
  Custom = OpenPitPretradeRejectCode_Custom,
  Other = OpenPitPretradeRejectCode_Other,
};

// A single pre-trade rejection record.
//
// Field order follows the native runtime `OpenPitPretradeReject` (largest
// first) so a view conversion stays mechanical. `userData` is an opaque
// caller-defined token the SDK never inspects; zero means unset.
struct Reject {
  std::string policy;
  std::string reason;
  std::string details;
  std::uintptr_t userData = 0;
  RejectCode code = RejectCode::Other;
  RejectScope scope = RejectScope::Order;

  Reject() = default;

  Reject(std::string policyName, RejectScope rejectScope, RejectCode rejectCode,
         std::string rejectReason, std::string rejectDetails)
      : policy(std::move(policyName)),
        reason(std::move(rejectReason)),
        details(std::move(rejectDetails)),
        code(rejectCode),
        scope(rejectScope) {}

  // Copies the borrowed string views out of a C reject record.
  [[nodiscard]] static Reject FromRaw(const OpenPitPretradeReject& raw) {
    Reject out;
    out.policy = ::openpit::StringView(raw.policy).ToString();
    out.reason = ::openpit::StringView(raw.reason).ToString();
    out.details = ::openpit::StringView(raw.details).ToString();
    out.userData = reinterpret_cast<std::uintptr_t>(raw.user_data);
    out.code = static_cast<RejectCode>(raw.code);
    out.scope = static_cast<RejectScope>(raw.scope);
    return out;
  }

  // Builds a C reject record whose string views borrow this object's strings;
  // valid only while this `Reject` is alive and unchanged.
  [[nodiscard]] OpenPitPretradeReject Raw() const noexcept {
    OpenPitPretradeReject raw{};
    raw.policy = ::openpit::MakeStringView(policy);
    raw.reason = ::openpit::MakeStringView(reason);
    raw.details = ::openpit::MakeStringView(details);
    raw.user_data = reinterpret_cast<void*>(userData);
    raw.code = static_cast<OpenPitPretradeRejectCode>(
        static_cast<std::uint16_t>(code));
    raw.scope = static_cast<OpenPitPretradeRejectScope>(
        static_cast<std::uint8_t>(scope));
    return raw;
  }
};

// Accumulator handed to a policy: a policy reports zero or more rejects into it
// during a pre-trade check. A non-empty decision means the order is rejected.
struct PolicyDecision {
  std::vector<Reject> rejects;

  [[nodiscard]] bool IsRejected() const noexcept { return !rejects.empty(); }

  void Push(Reject reject) { rejects.push_back(std::move(reject)); }
};

}  // namespace openpit::pretrade

namespace openpit::reject {

// Convenience re-exports of the canonical reject types from
// `openpit::pretrade`.
using RejectScope = ::openpit::pretrade::RejectScope;
using RejectCode = ::openpit::pretrade::RejectCode;
using Reject = ::openpit::pretrade::Reject;
using PolicyDecision = ::openpit::pretrade::PolicyDecision;

}  // namespace openpit::reject
