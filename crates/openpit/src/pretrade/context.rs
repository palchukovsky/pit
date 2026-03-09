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

use crate::param::CashFlow;

use crate::core::Order;

/// Immutable request data available during main-stage pre-trade checks.
///
/// `notional` is precomputed before policy execution. The context is provided
/// by the engine to [`crate::pretrade::Policy::perform_pre_trade_check`]; it cannot be constructed
/// directly by user code.
pub struct Context<'a> {
    order: &'a Order,
    notional: CashFlow,
}

impl<'a> Context<'a> {
    /// Returns the current order.
    ///
    /// # Examples
    ///
    /// ```
    /// use openpit::pretrade::{Context, Mutations, Policy, Reject};
    ///
    /// struct InspectPolicy;
    ///
    /// impl Policy for InspectPolicy {
    ///     fn name(&self) -> &'static str { "InspectPolicy" }
    ///
    ///     fn perform_pre_trade_check(
    ///         &self,
    ///         ctx: &Context<'_>,
    ///         _mutations: &mut Mutations,
    ///         _rejects: &mut Vec<Reject>,
    ///     ) {
    ///         let _ = ctx.order().instrument.settlement_asset();
    ///     }
    /// }
    /// ```
    pub fn order(&self) -> &Order {
        self.order
    }

    /// Returns the precomputed notional.
    ///
    /// Negative `CashFlow` means outflow (buy); positive means inflow (sell).
    ///
    /// # Examples
    ///
    /// ```
    /// use openpit::pretrade::{Context, Mutations, Policy, Reject};
    ///
    /// struct NotionalLogger;
    ///
    /// impl Policy for NotionalLogger {
    ///     fn name(&self) -> &'static str { "NotionalLogger" }
    ///
    ///     fn perform_pre_trade_check(
    ///         &self,
    ///         ctx: &Context<'_>,
    ///         _mutations: &mut Mutations,
    ///         _rejects: &mut Vec<Reject>,
    ///     ) {
    ///         let _ = ctx.notional();
    ///     }
    /// }
    /// ```
    pub fn notional(&self) -> CashFlow {
        self.notional
    }

    pub(crate) fn new(order: &'a Order, notional: CashFlow) -> Self {
        Self { order, notional }
    }
}

#[cfg(test)]
mod tests {
    use crate::param::{Asset, CashFlow, Price, Quantity, Side};

    use crate::core::{Instrument, Order};

    use super::Context;

    #[test]
    fn accessors_return_original_order_and_notional() {
        let order = Order {
            instrument: Instrument::new(
                Asset::new("AAPL").expect("asset code must be valid"),
                Asset::new("USD").expect("asset code must be valid"),
            ),
            side: Side::Buy,
            quantity: Quantity::from_str("10").expect("quantity must be valid"),
            price: Price::from_str("100").expect("price must be valid"),
        };
        let notional = CashFlow::from_str("-1000").expect("cash flow must be valid");
        let ctx = Context::new(&order, notional);

        assert_eq!(ctx.order(), &order);
        assert_eq!(ctx.notional(), notional);
    }
}
