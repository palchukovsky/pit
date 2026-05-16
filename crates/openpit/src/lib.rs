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

//! Embeddable pre-trade risk engine for trading systems.
//!
//! `openpit` focuses on the moment before an order leaves the application.
//! The crate provides:
//!
//! - [`Engine`] to coordinate pre-trade checks and post-trade feedback;
//! - [`param`] for typed financial values such as [`param::Price`] and
//!   [`param::Pnl`];
//! - [`pretrade`] for policy traits, rejects, deferred requests, and
//!   reservations.
//!
//! The pipeline is intentionally explicit:
//!
//! 1. [`Engine::start_pre_trade`] runs start-stage policies and returns a
//!    [`pretrade::PreTradeRequest`].
//! 2. [`pretrade::PreTradeRequest::execute`] runs main-stage policies and returns a
//!    [`pretrade::PreTradeReservation`].
//! 3. [`pretrade::PreTradeReservation::commit`] or
//!    [`pretrade::PreTradeReservation::rollback`] finalizes reserved state.
//! 4. [`Engine::apply_execution_report`] feeds realized outcomes back into policies.
//! 5. [`Engine::apply_account_adjustment`] validates non-trade adjustment batches.
//!
//! The current crate scope is deliberately narrow: in-memory admission control,
//! exact decimal value types, and a small set of built-in start-stage
//! policies. Persistence, market connectivity, and thread synchronization stay
//! with the caller.

mod core;
pub mod param;
pub mod pretrade;
pub mod storage;

pub use core::engine::{
    AccountAdjustmentBatchError, Engine, EngineBuildError, EngineBuilder, LocalEngine,
    SequentialEngine, SyncedEngine,
};
pub use core::{
    AccountAdjustmentAmount, AccountAdjustmentBalanceOperation, AccountAdjustmentBounds,
    AccountAdjustmentContext, AccountAdjustmentPositionOperation, AccountKey, AccountKeyConstraint,
    ExecutionReportFillDetails, ExecutionReportOperation, ExecutionReportPositionImpact,
    FinancialImpact, HasAccountAdjustmentBalanceAverageEntryPrice, HasAccountAdjustmentPending,
    HasAccountAdjustmentPendingLowerBound, HasAccountAdjustmentPendingUpperBound,
    HasAccountAdjustmentPositionLeverage, HasAccountAdjustmentReserved,
    HasAccountAdjustmentReservedLowerBound, HasAccountAdjustmentReservedUpperBound,
    HasAccountAdjustmentTotal, HasAccountAdjustmentTotalLowerBound,
    HasAccountAdjustmentTotalUpperBound, HasAccountId, HasAutoBorrow, HasAverageEntryPrice,
    HasBalanceAsset, HasClosePosition, HasCollateralAsset, HasExecutionReportIsFinal,
    HasExecutionReportLastTrade, HasExecutionReportPositionEffect, HasExecutionReportPositionSide,
    HasFee, HasInstrument, HasLeavesQuantity, HasLock, HasOrderCollateralAsset, HasOrderLeverage,
    HasOrderPositionSide, HasOrderPrice, HasPnl, HasPositionInstrument, HasPositionMode,
    HasReduceOnly, HasSide, HasTradeAmount, Instrument, Mutation, Mutations, OrderMargin,
    OrderOperation, OrderPosition, RequestFieldAccessError, WithAccountAdjustmentAmount,
    WithAccountAdjustmentBalanceOperation, WithAccountAdjustmentBounds,
    WithAccountAdjustmentPositionOperation, WithExecutionReportFillDetails,
    WithExecutionReportOperation, WithExecutionReportPositionImpact, WithFinancialImpact,
    WithOrderMargin, WithOrderOperation, WithOrderPosition,
};
pub use core::{
    AccountSyncPolicy, FullSyncPolicy, LocalSyncPolicy, ReadyEngineBuilder, SyncPolicy,
    SyncedEngineBuilder,
};
pub use core::{
    EngineLockingPolicy, LocalEngineLocking, SequentialEngineLocking, SyncedEngineLocking,
};
#[cfg(feature = "derive")]
pub use openpit_derive::RequestFields;
pub use param::{AdjustmentAmount, PositionMode};
pub use pretrade::PostTradeResult;
pub use storage::StorageBuilder;

/// Workspace-private re-exports. NOT part of the public API.
///
/// Workspace crates (e.g. `pit-interop`) that need to extend the
/// engine's locking strategy with their own `EngineLockingPolicy`
/// impl reach internal openpit traits through this module. Out-of-
/// workspace consumers should not depend on this path; its contents
/// are unstable and undocumented.
#[doc(hidden)]
pub mod __private {
    pub use crate::core::engine::EnginePolicies;

    /// Internal sealed-trait marker. Not part of the public API.
    ///
    /// Workspace crates implementing a custom [`super::EngineLockingPolicy`] must
    /// add `impl openpit::__private::Sealed for MyType {}` to opt in. External
    /// crates cannot reach this trait.
    pub trait Sealed {}
}
