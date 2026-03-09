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
// Please see https://github.com/openpitkit and the OWNERS file for details.

use std::fmt::{Display, Formatter};

/// Reject scope returned by policies.
///
/// # Examples
///
/// ```
/// use openpit::pretrade::RejectScope;
///
/// let scope = RejectScope::Order;
/// match scope {
///     RejectScope::Order => { /* retry is safe; engine remains operational */ }
///     RejectScope::Account => { /* halt trading until situation is resolved */ }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RejectScope {
    /// Reject only the current order.
    Order,
    /// Account-level reject signal.
    ///
    /// Engine reports it; application decides whether to stop trading.
    Account,
}

/// Standardized reject code for blocked orders.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RejectCode {
    /// A mandatory order field was not provided.
    MissingRequiredField,
    /// A field format is syntactically invalid.
    InvalidFieldFormat,
    /// A field value is outside accepted domain values.
    InvalidFieldValue,
    /// The order type is not supported.
    UnsupportedOrderType,
    /// The time-in-force value is not supported.
    UnsupportedTimeInForce,
    /// A requested order attribute is unsupported.
    UnsupportedOrderAttribute,
    /// Client order ID is already in use.
    DuplicateClientOrderId,
    /// Order arrival is outside the allowed entry window.
    TooLateToEnter,
    /// Venue session is closed for trading.
    ExchangeClosed,
    /// Instrument identifier is not recognized.
    UnknownInstrument,
    /// Account identifier is not recognized.
    UnknownAccount,
    /// Venue identifier is not recognized.
    UnknownVenue,
    /// Clearing account is not recognized.
    UnknownClearingAccount,
    /// Collateral asset is not recognized.
    UnknownCollateralAsset,
    /// Available cash is not sufficient for this order.
    InsufficientFunds,
    /// Margin is insufficient for this order.
    InsufficientMargin,
    /// Position inventory is insufficient for this order.
    InsufficientPosition,
    /// Credit limit is exceeded.
    CreditLimitExceeded,
    /// A generic risk limit is exceeded.
    RiskLimitExceeded,
    /// Multiple size limits are exceeded by one order.
    OrderExceedsLimit,
    /// Quantity limit is exceeded.
    OrderQtyExceedsLimit,
    /// Notional limit is exceeded.
    OrderNotionalExceedsLimit,
    /// Position limit is exceeded.
    PositionLimitExceeded,
    /// Concentration limit is exceeded.
    ConcentrationLimitExceeded,
    /// Leverage limit is exceeded.
    LeverageLimitExceeded,
    /// Rate limit for order submissions is exceeded.
    RateLimitExceeded,
    /// PnL-based kill switch is currently triggered.
    PnlKillSwitchTriggered,
    /// Account is blocked from trading.
    AccountBlocked,
    /// Account is not authorized to place this order.
    AccountNotAuthorized,
    /// Compliance rule forbids this order.
    ComplianceRestriction,
    /// Instrument is restricted for this account or venue.
    InstrumentRestricted,
    /// Jurisdictional restriction forbids this order.
    JurisdictionRestriction,
    /// Wash-trade prevention blocked this order.
    WashTradePrevention,
    /// Self-match prevention blocked this order.
    SelfMatchPrevention,
    /// Short-sale restriction blocked this order.
    ShortSaleRestriction,
    /// Required risk configuration is missing.
    RiskConfigurationMissing,
    /// Required reference data is unavailable.
    ReferenceDataUnavailable,
    /// Order value could not be calculated.
    OrderValueCalculationFailed,
    /// Risk system is temporarily unavailable.
    SystemUnavailable,
    /// Reject reason does not fit a more specific code.
    Other,
}

impl RejectCode {
    /// Returns the stable string representation of this code.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingRequiredField => "MissingRequiredField",
            Self::InvalidFieldFormat => "InvalidFieldFormat",
            Self::InvalidFieldValue => "InvalidFieldValue",
            Self::UnsupportedOrderType => "UnsupportedOrderType",
            Self::UnsupportedTimeInForce => "UnsupportedTimeInForce",
            Self::UnsupportedOrderAttribute => "UnsupportedOrderAttribute",
            Self::DuplicateClientOrderId => "DuplicateClientOrderId",
            Self::TooLateToEnter => "TooLateToEnter",
            Self::ExchangeClosed => "ExchangeClosed",
            Self::UnknownInstrument => "UnknownInstrument",
            Self::UnknownAccount => "UnknownAccount",
            Self::UnknownVenue => "UnknownVenue",
            Self::UnknownClearingAccount => "UnknownClearingAccount",
            Self::UnknownCollateralAsset => "UnknownCollateralAsset",
            Self::InsufficientFunds => "InsufficientFunds",
            Self::InsufficientMargin => "InsufficientMargin",
            Self::InsufficientPosition => "InsufficientPosition",
            Self::CreditLimitExceeded => "CreditLimitExceeded",
            Self::RiskLimitExceeded => "RiskLimitExceeded",
            Self::OrderExceedsLimit => "OrderExceedsLimit",
            Self::OrderQtyExceedsLimit => "OrderQtyExceedsLimit",
            Self::OrderNotionalExceedsLimit => "OrderNotionalExceedsLimit",
            Self::PositionLimitExceeded => "PositionLimitExceeded",
            Self::ConcentrationLimitExceeded => "ConcentrationLimitExceeded",
            Self::LeverageLimitExceeded => "LeverageLimitExceeded",
            Self::RateLimitExceeded => "RateLimitExceeded",
            Self::PnlKillSwitchTriggered => "PnlKillSwitchTriggered",
            Self::AccountBlocked => "AccountBlocked",
            Self::AccountNotAuthorized => "AccountNotAuthorized",
            Self::ComplianceRestriction => "ComplianceRestriction",
            Self::InstrumentRestricted => "InstrumentRestricted",
            Self::JurisdictionRestriction => "JurisdictionRestriction",
            Self::WashTradePrevention => "WashTradePrevention",
            Self::SelfMatchPrevention => "SelfMatchPrevention",
            Self::ShortSaleRestriction => "ShortSaleRestriction",
            Self::RiskConfigurationMissing => "RiskConfigurationMissing",
            Self::ReferenceDataUnavailable => "ReferenceDataUnavailable",
            Self::OrderValueCalculationFailed => "OrderValueCalculationFailed",
            Self::SystemUnavailable => "SystemUnavailable",
            Self::Other => "Other",
        }
    }
}

impl Display for RejectCode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Single rejection record returned by checks.
///
/// # Examples
///
/// ```
/// use openpit::pretrade::{Reject, RejectCode, RejectScope};
///
/// let reject = Reject::new(
///     "RateLimitPolicy",
///     RejectScope::Order,
///     RejectCode::RateLimitExceeded,
///     "rate limit exceeded",
///     "submitted 3 orders in 1s window, max allowed: 2",
/// );
/// assert_eq!(reject.code, RejectCode::RateLimitExceeded);
/// assert_eq!(reject.reason, "rate limit exceeded");
/// assert_eq!(reject.details, "submitted 3 orders in 1s window, max allowed: 2");
/// assert_eq!(reject.policy, "RateLimitPolicy");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Reject {
    /// Stable machine-readable reject code.
    pub code: RejectCode,
    /// Human-readable reject reason.
    pub reason: String,
    /// Case-specific reject details.
    pub details: String,
    /// Policy name that produced the reject.
    pub policy: &'static str,
    /// Reject scope.
    pub scope: RejectScope,
}

impl Reject {
    /// Creates a reject with human-readable reason and details.
    pub fn new(
        policy: &'static str,
        scope: RejectScope,
        code: RejectCode,
        reason: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            code,
            reason: reason.into(),
            details: details.into(),
            policy,
            scope,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RejectCode;

    #[test]
    fn reject_code_as_str_and_display_are_stable_for_all_values() {
        let cases = [
            (RejectCode::MissingRequiredField, "MissingRequiredField"),
            (RejectCode::InvalidFieldFormat, "InvalidFieldFormat"),
            (RejectCode::InvalidFieldValue, "InvalidFieldValue"),
            (RejectCode::UnsupportedOrderType, "UnsupportedOrderType"),
            (RejectCode::UnsupportedTimeInForce, "UnsupportedTimeInForce"),
            (
                RejectCode::UnsupportedOrderAttribute,
                "UnsupportedOrderAttribute",
            ),
            (RejectCode::DuplicateClientOrderId, "DuplicateClientOrderId"),
            (RejectCode::TooLateToEnter, "TooLateToEnter"),
            (RejectCode::ExchangeClosed, "ExchangeClosed"),
            (RejectCode::UnknownInstrument, "UnknownInstrument"),
            (RejectCode::UnknownAccount, "UnknownAccount"),
            (RejectCode::UnknownVenue, "UnknownVenue"),
            (RejectCode::UnknownClearingAccount, "UnknownClearingAccount"),
            (RejectCode::UnknownCollateralAsset, "UnknownCollateralAsset"),
            (RejectCode::InsufficientFunds, "InsufficientFunds"),
            (RejectCode::InsufficientMargin, "InsufficientMargin"),
            (RejectCode::InsufficientPosition, "InsufficientPosition"),
            (RejectCode::CreditLimitExceeded, "CreditLimitExceeded"),
            (RejectCode::RiskLimitExceeded, "RiskLimitExceeded"),
            (RejectCode::OrderExceedsLimit, "OrderExceedsLimit"),
            (RejectCode::OrderQtyExceedsLimit, "OrderQtyExceedsLimit"),
            (
                RejectCode::OrderNotionalExceedsLimit,
                "OrderNotionalExceedsLimit",
            ),
            (RejectCode::PositionLimitExceeded, "PositionLimitExceeded"),
            (
                RejectCode::ConcentrationLimitExceeded,
                "ConcentrationLimitExceeded",
            ),
            (RejectCode::LeverageLimitExceeded, "LeverageLimitExceeded"),
            (RejectCode::RateLimitExceeded, "RateLimitExceeded"),
            (RejectCode::PnlKillSwitchTriggered, "PnlKillSwitchTriggered"),
            (RejectCode::AccountBlocked, "AccountBlocked"),
            (RejectCode::AccountNotAuthorized, "AccountNotAuthorized"),
            (RejectCode::ComplianceRestriction, "ComplianceRestriction"),
            (RejectCode::InstrumentRestricted, "InstrumentRestricted"),
            (
                RejectCode::JurisdictionRestriction,
                "JurisdictionRestriction",
            ),
            (RejectCode::WashTradePrevention, "WashTradePrevention"),
            (RejectCode::SelfMatchPrevention, "SelfMatchPrevention"),
            (RejectCode::ShortSaleRestriction, "ShortSaleRestriction"),
            (
                RejectCode::RiskConfigurationMissing,
                "RiskConfigurationMissing",
            ),
            (
                RejectCode::ReferenceDataUnavailable,
                "ReferenceDataUnavailable",
            ),
            (
                RejectCode::OrderValueCalculationFailed,
                "OrderValueCalculationFailed",
            ),
            (RejectCode::SystemUnavailable, "SystemUnavailable"),
            (RejectCode::Other, "Other"),
        ];

        for (code, expected_name) in cases {
            assert_eq!(code.as_str(), expected_name);
            assert_eq!(code.to_string(), expected_name);
        }
    }
}
