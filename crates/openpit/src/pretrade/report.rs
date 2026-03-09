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

use crate::core::Instrument;
use crate::param::{Fee, Pnl};

/// Post-trade execution input.
///
/// # Examples
///
/// ```
/// use openpit::param::{Asset, Fee, Pnl};
/// use openpit::core::Instrument;
/// use openpit::pretrade::ExecutionReport;
///
/// let report = ExecutionReport {
///     instrument: Instrument::new(
///         Asset::new("AAPL").expect("asset code must be valid"),
///         Asset::new("USD").expect("asset code must be valid"),
///     ),
///     pnl: Pnl::from_str("-50").expect("valid"),
///     fee: Fee::from_str("3").expect("valid"),
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionReport {
    /// Instrument of the executed trade.
    pub instrument: Instrument,
    /// Realized P&L value.
    ///
    /// In the current version this value is treated as final realized P&L for
    /// the report, including any fees already accounted for by the caller.
    pub pnl: Pnl,
    /// Fee value for transparency and forward compatibility.
    ///
    /// Callers that already included fees in `pnl` can pass [`Fee::ZERO`].
    pub fee: Fee,
}

/// Aggregated post-trade processing result.
///
/// # Examples
///
/// ```
/// use openpit::param::{Asset, Fee, Pnl};
/// use openpit::core::Instrument;
/// use openpit::pretrade::ExecutionReport;
/// use openpit::Engine;
///
/// let engine = Engine::builder().build().expect("valid config");
/// let report = ExecutionReport {
///     instrument: Instrument::new(
///         Asset::new("AAPL").expect("asset code must be valid"),
///         Asset::new("USD").expect("asset code must be valid"),
///     ),
///     pnl: Pnl::from_str("-50").expect("valid"),
///     fee: Fee::ZERO,
/// };
/// let result = engine.apply_execution_report(&report);
/// assert!(!result.kill_switch_triggered);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PostTradeResult {
    /// True if at least one policy reported a kill-switch trigger.
    pub kill_switch_triggered: bool,
}
