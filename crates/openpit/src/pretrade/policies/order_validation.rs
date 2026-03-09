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

use crate::core::Order;
use crate::pretrade::{CheckPreTradeStartPolicy, Reject, RejectCode, RejectScope};

/// Start-stage policy for basic order field validation.
///
/// The current implementation enforces only one rule:
/// order quantity must be non-zero.
///
/// Price can be zero or negative.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OrderValidationPolicy;

impl OrderValidationPolicy {
    /// Stable policy name.
    pub const NAME: &'static str = "OrderValidationPolicy";

    /// Creates a new order validation policy.
    pub fn new() -> Self {
        Self
    }
}

impl CheckPreTradeStartPolicy for OrderValidationPolicy {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn check_pre_trade_start(&self, order: &Order) -> Result<(), Reject> {
        if order.quantity.is_zero() {
            return Err(Reject::new(
                self.name(),
                RejectScope::Order,
                RejectCode::InvalidFieldValue,
                "order quantity must be non-zero",
                "requested quantity 0 is not allowed",
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{Instrument, Order};
    use crate::param::{Asset, Price, Quantity, Side};
    use crate::pretrade::{CheckPreTradeStartPolicy, RejectCode, RejectScope};

    use super::OrderValidationPolicy;

    #[test]
    fn rejects_zero_quantity() {
        let policy = OrderValidationPolicy::new();
        let order = Order {
            instrument: Instrument::new(
                Asset::new("AAPL").expect("asset code must be valid"),
                Asset::new("USD").expect("asset code must be valid"),
            ),
            side: Side::Buy,
            quantity: Quantity::ZERO,
            price: Price::from_str("10").expect("price must be valid"),
        };

        let reject = policy
            .check_pre_trade_start(&order)
            .expect_err("zero quantity must be rejected");
        assert_eq!(reject.scope, RejectScope::Order);
        assert_eq!(reject.code, RejectCode::InvalidFieldValue);
        assert_eq!(reject.reason, "order quantity must be non-zero");
        assert_eq!(reject.details, "requested quantity 0 is not allowed");
    }

    #[test]
    fn allows_zero_price() {
        let policy = OrderValidationPolicy::new();
        let order = Order {
            instrument: Instrument::new(
                Asset::new("AAPL").expect("asset code must be valid"),
                Asset::new("USD").expect("asset code must be valid"),
            ),
            side: Side::Buy,
            quantity: Quantity::from_str("1").expect("quantity must be valid"),
            price: Price::ZERO,
        };

        assert!(policy.check_pre_trade_start(&order).is_ok());
    }

    #[test]
    fn allows_negative_price() {
        let policy = OrderValidationPolicy::new();
        let order = Order {
            instrument: Instrument::new(
                Asset::new("AAPL").expect("asset code must be valid"),
                Asset::new("USD").expect("asset code must be valid"),
            ),
            side: Side::Buy,
            quantity: Quantity::from_str("1").expect("quantity must be valid"),
            price: Price::from_str("-5").expect("price must be valid"),
        };

        assert!(policy.check_pre_trade_start(&order).is_ok());
    }
}
