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

use std::cell::RefCell;
use std::collections::HashMap;

use crate::core::Order;
use crate::param::{Asset, Quantity, Volume};

use crate::pretrade::{CheckPreTradeStartPolicy, Reject, RejectCode, RejectScope};

/// Per-settlement order size limits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderSizeLimit {
    /// Maximum allowed order notional for the settlement asset.
    pub max_notional: Volume,
    /// Maximum allowed order quantity for the settlement asset.
    pub max_quantity: Quantity,
    /// Settlement asset the limit applies to.
    pub settlement_asset: Asset,
}

/// Start-stage policy enforcing per-settlement order size limits.
///
/// Limits are configured per settlement asset. Orders for assets without a
/// configured limit are always rejected with [`RejectScope::Order`].
///
/// # Examples
///
/// ```
/// use openpit::param::{Asset, Price, Quantity, Side, Volume};
/// use openpit::core::Instrument;
/// use openpit::pretrade::policies::{OrderSizeLimit, OrderSizeLimitPolicy};
/// use openpit::{Engine, Order};
///
/// let policy = OrderSizeLimitPolicy::new(
///     OrderSizeLimit {
///         settlement_asset: Asset::new("USD").expect("asset code must be valid"),
///         max_quantity: Quantity::from_str("100").expect("valid"),
///         max_notional: Volume::from_str("50000").expect("valid"),
///     },
///     [],
/// );
///
/// let engine = Engine::builder()
///     .check_pre_trade_start_policy(policy)
///     .build()
///     .expect("valid");
///
/// let order = Order {
///     instrument: Instrument::new(
///         Asset::new("AAPL").expect("asset code must be valid"),
///         Asset::new("USD").expect("asset code must be valid"),
///     ),
///     side: Side::Buy,
///     quantity: Quantity::from_str("10").expect("valid"),
///     price: Price::from_str("200").expect("valid"),
/// };
/// assert!(engine.start_pre_trade(order).is_ok());
/// ```
pub struct OrderSizeLimitPolicy {
    limits: RefCell<HashMap<Asset, OrderSizeLimit>>,
}

impl OrderSizeLimitPolicy {
    /// Stable policy name.
    pub const NAME: &'static str = "OrderSizeLimitPolicy";

    /// Creates an order-size policy with at least one configured limit.
    pub fn new(
        initial_limit: OrderSizeLimit,
        additional_limits: impl IntoIterator<Item = OrderSizeLimit>,
    ) -> Self {
        let mut limits = HashMap::new();
        limits.insert(initial_limit.settlement_asset.clone(), initial_limit);
        for limit in additional_limits {
            limits.insert(limit.settlement_asset.clone(), limit);
        }

        Self {
            limits: RefCell::new(limits),
        }
    }

    /// Registers or replaces a limit for `limit.settlement_asset`.
    pub fn set_limit(&self, limit: OrderSizeLimit) {
        self.limits
            .borrow_mut()
            .insert(limit.settlement_asset.clone(), limit);
    }
}

impl CheckPreTradeStartPolicy for OrderSizeLimitPolicy {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_pre_trade_start(&self, order: &Order) -> Result<(), Reject> {
        let settlement = order.instrument.settlement_asset();
        let limit = match self.limits.borrow().get(settlement).cloned() {
            Some(limit) => limit,
            None => {
                return Err(Reject::new(
                    self.name(),
                    RejectScope::Order,
                    RejectCode::RiskConfigurationMissing,
                    "order size limit missing",
                    format!("settlement asset {settlement} has no configured limit"),
                ));
            }
        };

        let quantity_exceeded = order.quantity > limit.max_quantity;
        let requested_notional = order.price.calculate_volume(order.quantity);
        let notional_exceeded = match &requested_notional {
            Ok(notional) => *notional > limit.max_notional,
            Err(_) => true,
        };

        match (quantity_exceeded, notional_exceeded) {
            (false, false) => Ok(()),
            (true, false) => Err(Reject::new(
                self.name(),
                RejectScope::Order,
                RejectCode::OrderQtyExceedsLimit,
                "order quantity exceeded",
                format!(
                    "requested {}, max allowed: {}",
                    order.quantity, limit.max_quantity
                ),
            )),
            (false, true) => Err(order_notional_reject(
                self.name(),
                order,
                &limit,
                requested_notional.as_ref().ok(),
            )),
            (true, true) => Err(order_size_reject(
                self.name(),
                order,
                &limit,
                requested_notional.as_ref().ok(),
            )),
        }
    }
}

fn order_notional_reject(
    policy: &'static str,
    order: &Order,
    limit: &OrderSizeLimit,
    requested_notional: Option<&Volume>,
) -> Reject {
    let details = match requested_notional {
        Some(notional) => format!(
            "requested {}, max allowed: {}",
            notional,
            limit.max_notional
        ),
        None => format!(
            "requested price {}, requested quantity {}, max allowed notional: {}; could not calculate requested notional",
            order.price,
            order.quantity,
            limit.max_notional
        ),
    };

    Reject::new(
        policy,
        RejectScope::Order,
        RejectCode::OrderNotionalExceedsLimit,
        "order notional exceeded",
        details,
    )
}

fn order_size_reject(
    policy: &'static str,
    order: &Order,
    limit: &OrderSizeLimit,
    requested_notional: Option<&Volume>,
) -> Reject {
    let notional_details = match requested_notional {
        Some(notional) => format!(
            "requested notional {}, max allowed: {}",
            notional,
            limit.max_notional
        ),
        None => format!(
            "requested price {}, requested quantity {}, max allowed notional: {}; could not calculate requested notional",
            order.price,
            order.quantity,
            limit.max_notional
        ),
    };

    Reject::new(
        policy,
        RejectScope::Order,
        RejectCode::OrderExceedsLimit,
        "order size exceeded",
        format!(
            "requested quantity {}, max allowed: {}; {notional_details}",
            order.quantity, limit.max_quantity
        ),
    )
}

#[cfg(test)]
mod tests {
    use crate::core::{Instrument, Order};
    use crate::param::{Asset, Price, Quantity, Side, Volume};
    use crate::pretrade::{CheckPreTradeStartPolicy, RejectCode, RejectScope};
    use rust_decimal::Decimal;

    use super::{OrderSizeLimit, OrderSizeLimitPolicy};

    #[test]
    fn quantity_violation_returns_order_quantity_exceeded() {
        let policy = OrderSizeLimitPolicy::new(limit("USD", "10", "1000"), []);

        let reject = policy
            .check_pre_trade_start(&order("USD", "11", "90"))
            .expect_err("quantity must be rejected");
        assert_eq!(reject.scope, RejectScope::Order);
        assert_eq!(reject.code, RejectCode::OrderQtyExceedsLimit);
        assert_eq!(reject.reason, "order quantity exceeded");
        assert_eq!(reject.details, "requested 11, max allowed: 10");
    }

    #[test]
    fn notional_violation_returns_order_notional_exceeded() {
        let policy = OrderSizeLimitPolicy::new(limit("USD", "10", "1000"), []);

        let reject = policy
            .check_pre_trade_start(&order("USD", "10", "101"))
            .expect_err("notional must be rejected");
        assert_eq!(reject.scope, RejectScope::Order);
        assert_eq!(reject.code, RejectCode::OrderNotionalExceedsLimit);
        assert_eq!(reject.reason, "order notional exceeded");
        assert_eq!(reject.details, "requested 1010, max allowed: 1000");
    }

    #[test]
    fn both_violations_are_returned_in_single_reject() {
        let policy = OrderSizeLimitPolicy::new(limit("USD", "10", "1000"), []);

        let reject = policy
            .check_pre_trade_start(&order("USD", "11", "100"))
            .expect_err("quantity and notional must be rejected");
        assert_eq!(reject.scope, RejectScope::Order);
        assert_eq!(reject.code, RejectCode::OrderExceedsLimit);
        assert_eq!(reject.reason, "order size exceeded");
        assert_eq!(
            reject.details,
            "requested quantity 11, max allowed: 10; requested notional 1100, max allowed: 1000"
        );
    }

    #[test]
    fn missing_limit_returns_order_size_limit_missing() {
        let policy = OrderSizeLimitPolicy::new(limit("EUR", "10", "1000"), []);

        let reject = policy
            .check_pre_trade_start(&order("USD", "1", "1"))
            .expect_err("missing limit must reject");
        assert_eq!(reject.scope, RejectScope::Order);
        assert_eq!(reject.code, RejectCode::RiskConfigurationMissing);
        assert_eq!(reject.reason, "order size limit missing");
        assert_eq!(
            reject.details,
            "settlement asset USD has no configured limit"
        );
    }

    #[test]
    fn boundary_values_are_accepted() {
        let policy = OrderSizeLimitPolicy::new(limit("USD", "10", "1000"), []);

        let result = policy.check_pre_trade_start(&order("USD", "10", "100"));
        assert!(result.is_ok());
    }

    #[test]
    fn unconfigured_settlement_rejects_when_limit_is_missing() {
        let policy = OrderSizeLimitPolicy::new(limit("EUR", "10", "1000"), []);

        let reject = policy
            .check_pre_trade_start(&order("USD", "1", "1"))
            .expect_err("default policy must reject without configured limits");
        assert_eq!(reject.code, RejectCode::RiskConfigurationMissing);
        assert_eq!(reject.reason, "order size limit missing");
        assert_eq!(
            reject.details,
            "settlement asset USD has no configured limit"
        );
    }

    #[test]
    fn volume_overflow_is_treated_as_notional_exceeded() {
        let policy = OrderSizeLimitPolicy::new(limit("USD", "100", "1000"), []);

        let order = Order {
            instrument: Instrument::new(
                Asset::new("AAPL").expect("asset code must be valid"),
                Asset::new("USD").expect("asset code must be valid"),
            ),
            side: Side::Buy,
            quantity: Quantity::from_str("2").expect("quantity literal must be valid"),
            price: Price::new(Decimal::MAX),
        };

        let reject = policy
            .check_pre_trade_start(&order)
            .expect_err("overflow must be treated as notional exceeded");
        assert_eq!(reject.scope, RejectScope::Order);
        assert_eq!(reject.code, RejectCode::OrderNotionalExceedsLimit);
        assert_eq!(reject.reason, "order notional exceeded");
        assert_eq!(
            reject.details,
            format!(
                "requested price {}, requested quantity 2, max allowed notional: 1000; could not calculate requested notional",
                Price::new(Decimal::MAX)
            )
        );
    }

    #[test]
    fn volume_overflow_with_quantity_violation_returns_order_size_exceeded() {
        let policy = OrderSizeLimitPolicy::new(limit("USD", "1", "1000"), []);

        let order = Order {
            instrument: Instrument::new(
                Asset::new("AAPL").expect("asset code must be valid"),
                Asset::new("USD").expect("asset code must be valid"),
            ),
            side: Side::Buy,
            quantity: Quantity::from_str("2").expect("quantity literal must be valid"),
            price: Price::new(Decimal::MAX),
        };

        let reject = policy
            .check_pre_trade_start(&order)
            .expect_err("overflow plus quantity violation must be order size exceeded");
        assert_eq!(reject.scope, RejectScope::Order);
        assert_eq!(reject.code, RejectCode::OrderExceedsLimit);
        assert_eq!(reject.reason, "order size exceeded");
        assert_eq!(
            reject.details,
            format!(
                "requested quantity 2, max allowed: 1; requested price {}, requested quantity 2, max allowed notional: 1000; could not calculate requested notional",
                Price::new(Decimal::MAX)
            )
        );
    }

    #[test]
    fn additional_limits_and_set_limit_are_applied() {
        let policy =
            OrderSizeLimitPolicy::new(limit("USD", "10", "1000"), [limit("EUR", "5", "500")]);

        assert!(policy
            .check_pre_trade_start(&order("EUR", "5", "100"))
            .is_ok());

        policy.set_limit(limit("EUR", "1", "100"));
        let reject = policy
            .check_pre_trade_start(&order("EUR", "2", "10"))
            .expect_err("updated limit must be enforced");
        assert_eq!(reject.code, RejectCode::OrderQtyExceedsLimit);
        assert_eq!(reject.details, "requested 2, max allowed: 1");
    }

    fn limit(settlement: &str, max_quantity: &str, max_notional: &str) -> OrderSizeLimit {
        OrderSizeLimit {
            max_notional: Volume::from_str(max_notional)
                .expect("max notional literal must be valid"),
            max_quantity: Quantity::from_str(max_quantity)
                .expect("max quantity literal must be valid"),
            settlement_asset: Asset::new(settlement).expect("asset code must be valid"),
        }
    }

    fn order(settlement: &str, quantity: &str, price: &str) -> Order {
        Order {
            instrument: Instrument::new(
                Asset::new("AAPL").expect("asset code must be valid"),
                Asset::new(settlement).expect("asset code must be valid"),
            ),
            side: Side::Buy,
            quantity: Quantity::from_str(quantity).expect("quantity literal must be valid"),
            price: Price::from_str(price).expect("price literal must be valid"),
        }
    }
}
