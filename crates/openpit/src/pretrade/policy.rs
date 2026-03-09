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

use super::{Context, ExecutionReport, Mutations, Reject};

/// Lightweight pre-trade policy executed in `start_pre_trade`.
///
/// Runs in registration order. The engine stops at the first reject and does not
/// proceed to subsequent policies in this stage. State changes made here are
/// never rolled back.
///
/// # Examples
///
/// ```
/// use openpit::core::Order;
/// use openpit::pretrade::{CheckPreTradeStartPolicy, ExecutionReport, Reject, RejectScope};
///
/// struct SessionPolicy {
///     active: std::cell::Cell<bool>,
/// }
///
/// impl CheckPreTradeStartPolicy for SessionPolicy {
///     fn name(&self) -> &'static str {
///         "SessionPolicy"
///     }
///
///     fn check_pre_trade_start(&self, _order: &Order) -> Result<(), Reject> {
///         if !self.active.get() {
///             return Err(Reject::new(
///                 self.name(),
///                 RejectScope::Account,
///                 openpit::pretrade::RejectCode::Other,
///                 "session inactive",
///                 "trading session is closed",
///             ));
///         }
///         Ok(())
///     }
/// }
/// ```
pub trait CheckPreTradeStartPolicy {
    /// Stable policy name.
    ///
    /// Must be unique across all policies registered in an engine.
    fn name(&self) -> &'static str;

    /// Performs lightweight checks before creating a request.
    fn check_pre_trade_start(&self, order: &Order) -> Result<(), Reject>;

    /// Applies post-trade updates.
    ///
    /// Returns `true` when this policy reports a kill-switch trigger.
    fn apply_execution_report(&self, _report: &ExecutionReport) -> bool {
        false
    }
}

/// Main pre-trade policy executed in `Request::execute`.
///
/// All registered policies run regardless of earlier rejects. Mutations are
/// registered as commit/rollback pairs. If any policy produces a reject, the
/// engine reverts all mutations in reverse registration order.
///
/// # Examples
///
/// ```
/// use openpit::pretrade::{Context, ExecutionReport, Mutations, Policy, Reject};
///
/// struct NoopPolicy;
///
/// impl Policy for NoopPolicy {
///     fn name(&self) -> &'static str {
///         "NoopPolicy"
///     }
///
///     fn perform_pre_trade_check(
///         &self,
///         _ctx: &Context<'_>,
///         _mutations: &mut Mutations,
///         _rejects: &mut Vec<Reject>,
///     ) {
///         // always passes
///     }
/// }
/// ```
pub trait Policy {
    /// Stable policy name.
    ///
    /// Must be unique across all policies registered in an engine.
    fn name(&self) -> &'static str;

    /// Evaluates request context and optionally emits rejects/mutations.
    fn perform_pre_trade_check(
        &self,
        ctx: &Context<'_>,
        mutations: &mut Mutations,
        rejects: &mut Vec<Reject>,
    );

    /// Applies post-trade updates.
    ///
    /// Returns `true` when this policy reports a kill-switch trigger.
    fn apply_execution_report(&self, _report: &ExecutionReport) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::param::{Asset, Fee, Pnl, Price, Quantity, Side, Volume};

    use crate::core::{Instrument, Order};

    use super::{CheckPreTradeStartPolicy, Context, ExecutionReport, Mutations, Policy};
    use crate::pretrade::Reject;

    struct StartPolicyNoop;

    impl CheckPreTradeStartPolicy for StartPolicyNoop {
        fn name(&self) -> &'static str {
            "StartPolicyNoop"
        }

        fn check_pre_trade_start(&self, _order: &Order) -> Result<(), Reject> {
            Ok(())
        }
    }

    struct MainPolicyNoop;

    impl Policy for MainPolicyNoop {
        fn name(&self) -> &'static str {
            "MainPolicyNoop"
        }

        fn perform_pre_trade_check(
            &self,
            _ctx: &Context<'_>,
            _mutations: &mut Mutations,
            _rejects: &mut Vec<Reject>,
        ) {
        }
    }

    #[test]
    fn default_post_trade_hooks_return_false() {
        let report = ExecutionReport {
            instrument: Instrument::new(
                Asset::new("AAPL").expect("asset code must be valid"),
                Asset::new("USD").expect("asset code must be valid"),
            ),
            pnl: Pnl::from_str("0").expect("pnl must be valid"),
            fee: Fee::ZERO,
        };

        assert!(!StartPolicyNoop.apply_execution_report(&report));
        assert!(!MainPolicyNoop.apply_execution_report(&report));
    }

    #[test]
    fn required_trait_methods_can_be_invoked_without_side_effects() {
        let order = Order {
            instrument: Instrument::new(
                Asset::new("AAPL").expect("asset code must be valid"),
                Asset::new("USD").expect("asset code must be valid"),
            ),
            side: Side::Buy,
            quantity: Quantity::from_str("1").expect("quantity must be valid"),
            price: Price::from_str("10").expect("price must be valid"),
        };
        let ctx = Context::new(
            &order,
            Volume::from_str("10")
                .expect("volume must be valid")
                .to_cash_flow_outflow(),
        );
        let mut mutations = Mutations::new();
        let mut rejects = Vec::new();

        assert_eq!(StartPolicyNoop.name(), "StartPolicyNoop");
        assert!(StartPolicyNoop.check_pre_trade_start(&order).is_ok());
        assert_eq!(MainPolicyNoop.name(), "MainPolicyNoop");
        MainPolicyNoop.perform_pre_trade_check(&ctx, &mut mutations, &mut rejects);
        assert!(mutations.as_slice().is_empty());
        assert!(rejects.is_empty());
    }
}
