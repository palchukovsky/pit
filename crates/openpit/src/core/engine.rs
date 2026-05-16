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

// EnginePolicies and PolicyBox are crate-private traits used in public bounds.
// This is intentional: external code uses only the built-in EL types (LocalEngineLocking,
// SyncedEngineLocking) which all implement these traits, so the bounds are always satisfied
// without the caller naming the traits.
#![allow(private_bounds)]

use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::time::Instant;

use super::engine_locking::{
    EngineLockingPolicy, LocalEngineLocking, SequentialEngineLocking, SyncedEngineLocking,
};
use super::sync_policy;
use crate::param::AccountId;
use crate::pretrade::handle::{RequestHandleImpl, ReservationHandleImpl};
use crate::pretrade::start_pre_trade_time::with_start_pre_trade_now;
use crate::pretrade::{
    PostTradeResult, PreTradeContext, PreTradePolicy, PreTradeRequest, PreTradeReservation, Reject,
    RejectCode, RejectScope, Rejects,
};
use crate::storage::StorageBuilder;
use crate::{AccountAdjustmentContext, Mutations};

// ─── Type aliases for per-EL policy Vecs ─────────────────────────────────────

type PreTradeVec<EL, Order, ExecutionReport, AccountAdjustment> =
    Vec<Box<<EL as EnginePolicies<Order, ExecutionReport, AccountAdjustment>>::PreTrade>>;

// ─── EnginePolicies ──────────────────────────────────────────────────────────

/// Internal trait that selects the dyn-trait-object shape for
/// registered policies on engines using a given [`EngineLockingPolicy`]
/// flavor.
///
/// This trait is intentionally `#[doc(hidden)]` and reachable from
/// workspace crates via [`openpit::__private::EnginePolicies`] only.
/// It is not part of the public API; implementations live in `openpit`
/// and `pit-interop` only.
#[doc(hidden)]
pub trait EnginePolicies<Order: 'static, ExecutionReport: 'static, AccountAdjustment: 'static>:
    EngineLockingPolicy
{
    type PreTrade: PreTradePolicy<Order, ExecutionReport, AccountAdjustment> + ?Sized + 'static;
}

impl<Order: 'static, ExecutionReport: 'static, AccountAdjustment: 'static>
    EnginePolicies<Order, ExecutionReport, AccountAdjustment> for LocalEngineLocking
{
    type PreTrade = dyn PreTradePolicy<Order, ExecutionReport, AccountAdjustment>;
}

impl<Order: 'static, ExecutionReport: 'static, AccountAdjustment: 'static>
    EnginePolicies<Order, ExecutionReport, AccountAdjustment> for SyncedEngineLocking
{
    type PreTrade = dyn PreTradePolicy<Order, ExecutionReport, AccountAdjustment> + Send + Sync;
}

impl<Order: 'static, ExecutionReport: 'static, AccountAdjustment: 'static>
    EnginePolicies<Order, ExecutionReport, AccountAdjustment> for SequentialEngineLocking
{
    type PreTrade = dyn PreTradePolicy<Order, ExecutionReport, AccountAdjustment> + Send;
}

// ─── PolicyBox ───────────────────────────────────────────────────────────────

/// Crate-private coercion helper: converts a concrete policy into a
/// `Box<Target>` where `Target` is the dyn type selected by [`EnginePolicies`].
///
/// Three blanket impls exist for the policy trait:
///
/// - `Target = dyn Trait` — satisfied by `Policy: Trait + 'static` (no
///   Send/Sync required; used for `LocalEngineLocking`).
/// - `Target = dyn Trait + Send` — satisfied by
///   `Policy: Trait + Send + 'static` (used for `SequentialEngineLocking`).
/// - `Target = dyn Trait + Send + Sync` — satisfied by
///   `Policy: Trait + Send + Sync + 'static` (used for `SyncedEngineLocking`).
///
/// The builder methods carry a single `where Policy: PolicyBox<EL::PreTrade>`
/// bound, which resolves to whichever blanket impl matches the concrete `Target`
/// for the active `EL`. This avoids duplicate method definitions while still
/// enforcing the right bounds per locking flavor.
#[doc(hidden)]
pub(crate) trait PolicyBox<Target: ?Sized>: 'static {
    fn into_box(self) -> Box<Target>;
}

#[doc(hidden)]
impl<
        Order: 'static,
        ExecutionReport: 'static,
        AccountAdjustment: 'static,
        PreTradePolicy: crate::pretrade::PreTradePolicy<Order, ExecutionReport, AccountAdjustment> + 'static,
    > PolicyBox<dyn crate::pretrade::PreTradePolicy<Order, ExecutionReport, AccountAdjustment>>
    for PreTradePolicy
{
    fn into_box(
        self,
    ) -> Box<dyn crate::pretrade::PreTradePolicy<Order, ExecutionReport, AccountAdjustment>> {
        Box::new(self)
    }
}

#[doc(hidden)]
impl<
        Order: 'static,
        ExecutionReport: 'static,
        AccountAdjustment: 'static,
        PreTradePolicy: crate::pretrade::PreTradePolicy<Order, ExecutionReport, AccountAdjustment> + Send + 'static,
    >
    PolicyBox<dyn crate::pretrade::PreTradePolicy<Order, ExecutionReport, AccountAdjustment> + Send>
    for PreTradePolicy
{
    fn into_box(
        self,
    ) -> Box<dyn crate::pretrade::PreTradePolicy<Order, ExecutionReport, AccountAdjustment> + Send>
    {
        Box::new(self)
    }
}

#[doc(hidden)]
impl<
        Order: 'static,
        ExecutionReport: 'static,
        AccountAdjustment: 'static,
        PreTradePolicy: crate::pretrade::PreTradePolicy<Order, ExecutionReport, AccountAdjustment>
            + Send
            + Sync
            + 'static,
    >
    PolicyBox<
        dyn crate::pretrade::PreTradePolicy<Order, ExecutionReport, AccountAdjustment>
            + Send
            + Sync,
    > for PreTradePolicy
{
    fn into_box(
        self,
    ) -> Box<
        dyn crate::pretrade::PreTradePolicy<Order, ExecutionReport, AccountAdjustment>
            + Send
            + Sync,
    > {
        Box::new(self)
    }
}

struct EngineInner<Order: 'static, ExecutionReport: 'static, AccountAdjustment: 'static, EL>
where
    EL: EngineLockingPolicy + EnginePolicies<Order, ExecutionReport, AccountAdjustment>,
{
    pre_trade_policies: Vec<Box<EL::PreTrade>>,
}

/// Errors returned by [`ReadyEngineBuilder::build`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EngineBuildError {
    /// Duplicate policy name across registered policy sets.
    DuplicatePolicyName { name: String },
}

impl Display for EngineBuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicatePolicyName { name } => {
                write!(formatter, "duplicate policy name: {name}")
            }
        }
    }
}

impl std::error::Error for EngineBuildError {}

/// Error returned when account-adjustment batch validation fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountAdjustmentBatchError {
    /// Rejects produced by the policy.
    pub rejects: Rejects,
    /// Zero-based index of the failing adjustment.
    pub failed_adjustment_index: usize,
}

impl Display for AccountAdjustmentBatchError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "account adjustment batch rejected at index {}: {}",
            self.failed_adjustment_index, self.rejects
        )
    }
}

impl std::error::Error for AccountAdjustmentBatchError {}

/// Risk engine orchestrating start-stage and main-stage pre-trade checks.
///
/// Build the engine once during platform initialization using
/// [`Engine::builder`], then share it across order submissions.
///
/// Generic parameters:
/// - `Order`: order contract type used by `start_pre_trade`;
/// - `ExecutionReport`: execution-report contract type used by `apply_execution_report`;
/// - `AccountAdjustment`: account-adjustment contract type used by `apply_account_adjustment`;
/// - `EL`: engine-locking policy; defaults to [`LocalEngineLocking`] (`!Send + !Sync`).
///   Use [`SyncedEngineLocking`] (produced by [`EngineBuilder::full_sync`]) for
///   `Send + Sync` engines, or [`SequentialEngineLocking`] (produced by
///   [`EngineBuilder::account_sync`]) for sequential cross-thread engines.
///
/// # Threading
///
/// The engine handle's thread-safety is determined by `EL`:
///
/// - [`LocalEngineLocking`] (default, produced by `no_sync`): the handle
///   is `!Send + !Sync`. Keep it on the OS thread that created it. Concurrent
///   invocation is not supported.
/// - [`SyncedEngineLocking`] (produced by `full_sync`): the handle is
///   `Send + Sync` when all registered policies are `Send + Sync`. It can be
///   wrapped in `Arc<Engine<..., SyncedEngineLocking>>` and shared across
///   threads. With `FullLocking` storage, concurrent invocation from multiple
///   threads is safe.
/// - [`SequentialEngineLocking`] (produced by `account_sync`): the handle
///   is `Send + !Sync`. Ownership may move between OS threads sequentially, but
///   concurrent invocation on the same handle is not supported.
///
/// Language bindings (Python, Go, C) may narrow this contract for their public
/// API surface — see the binding documentation for the exact rules each binding
/// offers.
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use openpit::param::{Asset, Price, Quantity, Side, TradeAmount};
/// use openpit::{Engine, Instrument, OrderOperation, WithOrderOperation};
/// use openpit::{FinancialImpact, ExecutionReportOperation, WithFinancialImpact, WithExecutionReportOperation};
/// use openpit::pretrade::policies::OrderValidationPolicy;
///
/// type MyOrder = WithOrderOperation<()>;
/// type MyReport = WithExecutionReportOperation<WithFinancialImpact<()>>;
///
/// let engine = Engine::<MyOrder, MyReport>::builder()
///     .no_sync()
///     .pre_trade(OrderValidationPolicy::new())
///     .build()?;
///
/// let order = WithOrderOperation {
///     inner: (),
///     operation: OrderOperation {
///         instrument: Instrument::new(Asset::new("AAPL")?, Asset::new("USD")?),
///         account_id: openpit::param::AccountId::from_u64(12345),
///         side: Side::Buy,
///         trade_amount: TradeAmount::Quantity(Quantity::from_f64(100.0)?),
///         price: Some(Price::from_str("185")?),
///     },
/// };
///
/// let request = engine.start_pre_trade(order)?;
/// let mut reservation = request.execute()?;
/// reservation.commit();
/// # Ok(())
/// # }
/// ```
pub struct Engine<
    Order: 'static,
    ExecutionReport: 'static = (),
    AccountAdjustment: 'static = (),
    Locking = LocalEngineLocking,
> where
    Locking: EngineLockingPolicy + EnginePolicies<Order, ExecutionReport, AccountAdjustment>,
{
    inner: Locking::Strong<EngineInner<Order, ExecutionReport, AccountAdjustment, Locking>>,
}

/// Single-threaded engine type alias.
///
/// Equivalent to `Engine<Order, ExecutionReport, AccountAdjustment, LocalEngineLocking>`.
/// Produced by [`EngineBuilder::no_sync`] chains.
pub type LocalEngine<Order, ExecutionReport = (), AccountAdjustment = ()> =
    Engine<Order, ExecutionReport, AccountAdjustment, LocalEngineLocking>;

/// Account-sharded engine type alias.
///
/// Equivalent to `Engine<Order, ExecutionReport, AccountAdjustment, SequentialEngineLocking>`.
/// Produced by [`EngineBuilder::account_sync`] chains. Engine handle is
/// `Send + !Sync`.
pub type SequentialEngine<Order, ExecutionReport = (), AccountAdjustment = ()> =
    Engine<Order, ExecutionReport, AccountAdjustment, SequentialEngineLocking>;

/// Multi-threaded engine type alias.
///
/// Equivalent to `Engine<Order, ExecutionReport, AccountAdjustment, SyncedEngineLocking>`.
/// Produced by [`EngineBuilder::full_sync`] chains. The resulting engine handle is
/// `Send + Sync` and may be wrapped in `Arc<SyncedEngine<...>>`.
pub type SyncedEngine<Order, ExecutionReport = (), AccountAdjustment = ()> =
    Engine<Order, ExecutionReport, AccountAdjustment, SyncedEngineLocking>;

/// # Threading
///
/// See [`Engine`]'s `# Threading` section for the threading contract.
impl<
        Order: 'static,
        ExecutionReport: 'static,
        AccountAdjustment: 'static,
        LockingPolicy: EngineLockingPolicy + EnginePolicies<Order, ExecutionReport, AccountAdjustment>,
    > Engine<Order, ExecutionReport, AccountAdjustment, LockingPolicy>
{
    /// Creates an engine builder.
    pub fn builder() -> EngineBuilder<Order, ExecutionReport, AccountAdjustment> {
        EngineBuilder::new()
    }

    /// Executes start-stage checks and creates a deferred [`PreTradeRequest`].
    ///
    /// Start-stage policies run in registration order. The engine collects
    /// reject lists returned by all start-stage policies before deciding
    /// whether to create a deferred request.
    ///
    /// The engine does not enforce optional order extensions (for example
    /// `instrument` or `side`). Policies that depend on extension fields must
    /// validate their presence.
    ///
    /// # Errors
    ///
    /// Returns [`Rejects`] when any start-stage policy rejects the order.
    pub fn start_pre_trade(&self, order: Order) -> Result<PreTradeRequest<Order>, Rejects> {
        let now: Instant = Instant::now();
        let ctx = PreTradeContext::new();
        let start_rejects = with_start_pre_trade_now(now, || {
            let inner: &EngineInner<Order, ExecutionReport, AccountAdjustment, LockingPolicy> =
                &self.inner;
            let mut lists = Vec::new();
            let mut len = 0;
            for policy in &inner.pre_trade_policies {
                if let Err(rejects) = policy.check_pre_trade_start(&ctx, &order) {
                    len += rejects.len();
                    lists.push(rejects);
                }
            }
            merge_reject_lists(lists, len)
        });
        if let Some(rejects) = start_rejects {
            return Err(rejects);
        }

        let engine = LockingPolicy::downgrade(&self.inner);
        let request_handle = RequestHandleImpl::<Order>::new(Box::new(move || {
            execute_request::<Order, ExecutionReport, AccountAdjustment, LockingPolicy>(
                engine, ctx, order,
            )
        }));

        Ok(PreTradeRequest::from_handle(Box::new(request_handle)))
    }

    /// Runs start-stage checks and executes main-stage checks immediately.
    ///
    /// This is a convenience shortcut equivalent to
    /// `engine.start_pre_trade(order)?.execute()`.
    ///
    /// # Errors
    ///
    /// Returns [`Rejects`] for both stages.
    pub fn execute_pre_trade(&self, order: Order) -> Result<PreTradeReservation, Rejects> {
        self.start_pre_trade(order)
            .and_then(PreTradeRequest::execute)
    }

    /// Applies post-trade updates and aggregates kill-switch status across all policies.
    ///
    /// Returns [`PostTradeResult::kill_switch_triggered`] `true` when at least one policy
    /// reports a kill-switch condition.
    pub fn apply_execution_report(&self, report: &ExecutionReport) -> PostTradeResult {
        let inner: &EngineInner<Order, ExecutionReport, AccountAdjustment, LockingPolicy> =
            &self.inner;
        let mut kill_switch_triggered = false;

        for policy in &inner.pre_trade_policies {
            kill_switch_triggered |= policy.apply_execution_report(report);
        }

        PostTradeResult {
            kill_switch_triggered,
        }
    }

    /// Validates an account-adjustment batch atomically.
    ///
    /// Policies are evaluated in registration order for each adjustment, and
    /// adjustments are traversed in slice order.
    ///
    /// # Errors
    ///
    /// Returns [`AccountAdjustmentBatchError`] for the first rejected element.
    /// The `index` field points to the failing adjustment in `adjustments`.
    pub fn apply_account_adjustment(
        &self,
        account_id: AccountId,
        adjustments: &[AccountAdjustment],
    ) -> Result<(), AccountAdjustmentBatchError> {
        if adjustments.is_empty() {
            return Ok(());
        }

        let inner: &EngineInner<Order, ExecutionReport, AccountAdjustment, LockingPolicy> =
            &self.inner;
        let mut mutations = Mutations::new();
        let mut batch_error: Option<AccountAdjustmentBatchError> = None;
        let ctx = AccountAdjustmentContext::new();

        'outer: for (index, adjustment) in adjustments.iter().enumerate() {
            for policy in &inner.pre_trade_policies {
                if let Err(rejects) =
                    policy.apply_account_adjustment(&ctx, account_id, adjustment, &mut mutations)
                {
                    if rejects.is_empty() {
                        continue;
                    }
                    batch_error = Some(AccountAdjustmentBatchError {
                        failed_adjustment_index: index,
                        rejects,
                    });
                    break 'outer;
                }
            }
        }

        if let Some(err) = batch_error {
            mutations.rollback_all();
            return Err(err);
        }

        mutations.commit_all();
        Ok(())
    }
}

/// Fluent builder for [`Engine`].
///
/// Policies are evaluated in registration order. Policy names must be unique
/// across start-stage, main-stage, and account-adjustment sets;
/// [`ReadyEngineBuilder::build`] returns [`EngineBuildError::DuplicatePolicyName`]
/// otherwise.
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use std::time::Duration;
/// use openpit::{WithExecutionReportOperation, WithFinancialImpact, WithOrderOperation};
/// use openpit::pretrade::policies::{
///     PnlBoundsAccountAssetBarrier, PnlBoundsBrokerBarrier, PnlBoundsKillSwitchPolicy,
///     RateLimit, RateLimitBrokerBarrier, RateLimitPolicy,
/// };
/// use openpit::Engine;
/// use openpit::param::{AccountId, Asset, Pnl};
///
/// type MyOrder = WithOrderOperation<()>;
/// type MyReport = WithFinancialImpact<WithExecutionReportOperation<()>>;
///
/// let builder = Engine::<MyOrder, MyReport>::builder().no_sync();
///
/// let pnl_policy = PnlBoundsKillSwitchPolicy::new(
///     [PnlBoundsBrokerBarrier {
///         settlement_asset: Asset::new("USD")?,
///         lower_bound: Some(Pnl::from_str("-500")?),
///         upper_bound: None,
///     }],
///     [PnlBoundsAccountAssetBarrier {
///         barrier: PnlBoundsBrokerBarrier {
///             settlement_asset: Asset::new("USD")?,
///             lower_bound: Some(Pnl::from_str("-200")?),
///             upper_bound: None,
///         },
///         account_id: AccountId::from_u64(99224416),
///         initial_pnl: Pnl::from_str("-50")?,
///     }],
///     builder.storage_builder(),
/// )?;
///
/// let rate_policy = RateLimitPolicy::new(
///     Some(RateLimitBrokerBarrier {
///         limit: RateLimit { max_orders: 100, window: Duration::from_secs(1) },
///     }),
///     [],
///     [],
///     [],
///     builder.storage_builder(),
/// )?;
///
/// let engine = builder
///     .pre_trade(pnl_policy)
///     .pre_trade(rate_policy)
///     .build()?;
/// let _ = engine;
/// # Ok(())
/// # }
/// ```
pub struct EngineBuilder<Order, ExecutionReport = (), AccountAdjustment = ()> {
    pre_trade: Vec<Box<dyn PreTradePolicy<Order, ExecutionReport, AccountAdjustment>>>,
}

impl<Order, ExecutionReport, AccountAdjustment>
    EngineBuilder<Order, ExecutionReport, AccountAdjustment>
{
    /// Creates a new builder.
    ///
    /// This is a crate-internal constructor. External callers must use
    /// [`Engine::builder`] as the entry point.
    #[allow(clippy::new_without_default)]
    pub(crate) fn new() -> Self {
        Self {
            pre_trade: Vec::new(),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn pre_trade<Policy>(mut self, policy: Policy) -> Self
    where
        Policy: PreTradePolicy<Order, ExecutionReport, AccountAdjustment> + 'static,
    {
        self.pre_trade.push(Box::new(policy));
        self
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn build(
        self,
    ) -> Result<
        Engine<Order, ExecutionReport, AccountAdjustment, LocalEngineLocking>,
        EngineBuildError,
    >
    where
        Order: 'static,
        ExecutionReport: 'static,
        AccountAdjustment: 'static,
    {
        ensure_unique_policy_names(self.pre_trade.iter().map(|p| p.name()))?;

        Ok(Engine {
            inner: LocalEngineLocking::new_strong(EngineInner {
                pre_trade_policies: self.pre_trade,
            }),
        })
    }

    /// Applies a custom synchronization policy and advances to
    /// [`SyncedEngineBuilder`].
    ///
    /// The policy is specified as a type argument only — the struct must
    /// implement [`SyncPolicy`] and is typically zero-sized. For the
    /// built-in regimes, prefer [`full_sync`](Self::full_sync),
    /// [`no_sync`](Self::no_sync), or
    /// [`account_sync`](Self::account_sync).
    ///
    /// [`SyncPolicy`]: crate::SyncPolicy
    pub fn sync<SyncPolicy>(
        self,
        policy: SyncPolicy,
    ) -> SyncedEngineBuilder<Order, ExecutionReport, AccountAdjustment, SyncPolicy>
    where
        SyncPolicy: sync_policy::SyncPolicy,
        SyncPolicy::EngineLocking: EnginePolicies<Order, ExecutionReport, AccountAdjustment>,
    {
        // Policies registered on EngineBuilder (if any, via private test helpers) are
        // intentionally dropped here: sync() establishes the locking flavor, and
        // subsequent policy registration happens on the returned SyncedEngineBuilder.
        // In production usage sync() is called immediately after builder(), so
        // this is always an empty discard.
        let _ = self;
        SyncedEngineBuilder {
            pre_trade_policies: Vec::new(),
            storage_builder: StorageBuilder::new(policy.create_locking_factory()),
            _marker: PhantomData,
        }
    }

    /// Applies full thread-safety synchronization and advances to
    /// [`SyncedEngineBuilder`].
    ///
    /// Storage tables created by registered policies will use
    /// [`FullLocking`]: index and value domains are each protected by an
    /// independent reader-writer lock.
    ///
    /// [`FullLocking`]: crate::storage::FullLocking
    pub fn full_sync(
        self,
    ) -> SyncedEngineBuilder<Order, ExecutionReport, AccountAdjustment, sync_policy::FullSyncPolicy>
    {
        self.sync(sync_policy::FullSyncPolicy)
    }

    /// Applies single-thread (no-sync) synchronization and advances to
    /// [`SyncedEngineBuilder`].
    ///
    /// Storage tables created by registered policies will use
    /// [`NoLocking`]: no synchronization primitives are allocated. The
    /// resulting storages are `!Send + !Sync`; this option is for
    /// single-threaded embeddings where synchronization overhead must be
    /// zero.
    ///
    /// [`NoLocking`]: crate::storage::NoLocking
    pub fn no_sync(
        self,
    ) -> SyncedEngineBuilder<Order, ExecutionReport, AccountAdjustment, sync_policy::LocalSyncPolicy>
    {
        self.sync(sync_policy::LocalSyncPolicy)
    }

    /// Applies account-index synchronization and advances to
    /// [`SyncedEngineBuilder`].
    ///
    /// Storage tables created by registered policies will use
    /// [`IndexLocking`]: one reader-writer lock guards key insertions and
    /// removals; per-value access is the caller's responsibility. The engine
    /// handle is `Send + !Sync`: ownership may move between OS threads
    /// sequentially, but concurrent invocation on the same handle is not
    /// supported.
    ///
    /// [`IndexLocking`]: crate::storage::IndexLocking
    pub fn account_sync(
        self,
    ) -> SyncedEngineBuilder<
        Order,
        ExecutionReport,
        AccountAdjustment,
        sync_policy::AccountSyncPolicy,
    > {
        self.sync(sync_policy::AccountSyncPolicy)
    }
}

// ─── SyncedEngineBuilder ─────────────────────────────────────────────────────

/// Engine builder with a synchronization policy applied.
///
/// Obtained from [`EngineBuilder::sync`], [`EngineBuilder::full_sync`],
/// [`EngineBuilder::no_sync`], or [`EngineBuilder::account_sync`].
///
/// This builder deliberately has **no `build` method**: at least one policy
/// must be registered before the engine can be constructed. Adding any policy
/// advances to [`ReadyEngineBuilder`], which exposes [`build`](ReadyEngineBuilder::build).
///
/// The `SyncPolicy` type parameter carries the chosen [`SyncPolicy`]
/// forward through the builder chain so that trading policies can create
/// correctly-synchronized [`Storage`] tables without knowing the concrete
/// factory type.
///
/// [`SyncPolicy`]: crate::SyncPolicy
/// [`Storage`]: crate::storage::Storage
pub struct SyncedEngineBuilder<
    Order: 'static,
    ExecutionReport: 'static,
    AccountAdjustment: 'static,
    SyncPolicy,
> where
    SyncPolicy: sync_policy::SyncPolicy,
    SyncPolicy::EngineLocking: EnginePolicies<Order, ExecutionReport, AccountAdjustment>,
{
    pre_trade_policies:
        PreTradeVec<SyncPolicy::EngineLocking, Order, ExecutionReport, AccountAdjustment>,
    storage_builder:
        StorageBuilder<<SyncPolicy as sync_policy::SyncPolicy>::StorageLockingPolicyFactory>,
    _marker: PhantomData<(Order, ExecutionReport, AccountAdjustment, SyncPolicy)>,
}

impl<Order, ExecutionReport, AccountAdjustment, SyncPolicyT>
    SyncedEngineBuilder<Order, ExecutionReport, AccountAdjustment, SyncPolicyT>
where
    SyncPolicyT: sync_policy::SyncPolicy,
    SyncPolicyT::EngineLocking: EnginePolicies<Order, ExecutionReport, AccountAdjustment>,
{
    /// Returns the storage builder owned by this engine builder. Pass it (or
    /// a borrowed reference to it) to policy constructors that need internal
    /// storage tables. The factory type is shared with the engine builder's
    /// synchronization policy.
    pub fn storage_builder(
        &self,
    ) -> &StorageBuilder<<SyncPolicyT as sync_policy::SyncPolicy>::StorageLockingPolicyFactory>
    {
        &self.storage_builder
    }
}

impl<Order: 'static, ExecutionReport: 'static, AccountAdjustment: 'static, SyncPolicyT>
    SyncedEngineBuilder<Order, ExecutionReport, AccountAdjustment, SyncPolicyT>
where
    SyncPolicyT: sync_policy::SyncPolicy,
    SyncPolicyT::EngineLocking: EnginePolicies<Order, ExecutionReport, AccountAdjustment>,
{
    /// Registers a policy and advances to [`ReadyEngineBuilder`].
    ///
    /// The required bound on `Policy` is determined by the `SyncPolicy`'s locking
    /// flavor:
    ///
    /// - `LocalEngineLocking` (from `no_sync`): `'static` only; `!Send`
    ///   policy state is accepted.
    /// - `SequentialEngineLocking` (from `account_sync`): `Send + 'static`.
    /// - `SyncedEngineLocking` (from `full_sync`): `Send + Sync + 'static`.
    pub fn pre_trade<Policy>(
        mut self,
        policy: Policy,
    ) -> ReadyEngineBuilder<Order, ExecutionReport, AccountAdjustment, SyncPolicyT>
    where
        Policy: PolicyBox<
            <SyncPolicyT::EngineLocking as EnginePolicies<
                Order,
                ExecutionReport,
                AccountAdjustment,
            >>::PreTrade,
        >,
    {
        self.pre_trade_policies.push(PolicyBox::into_box(policy));
        ReadyEngineBuilder {
            pre_trade_policies: self.pre_trade_policies,
            storage_builder: self.storage_builder,
            _marker: PhantomData,
        }
    }
}

// ─── ReadyEngineBuilder ──────────────────────────────────────────────────────

/// Engine builder with a synchronization policy and at least one trading
/// policy registered. Can produce an [`Engine`] via [`build`](Self::build).
///
/// Obtained from the `add_policy` methods on [`SyncedEngineBuilder`] or
/// from the chained `add_policy` methods on this type itself.
///
/// The `SyncPolicy` type parameter carries the chosen [`SyncPolicy`]
/// to any code that needs to create additional [`Storage`] tables with the
/// same synchronization regime.
///
/// [`SyncPolicy`]: crate::SyncPolicy
/// [`Storage`]: crate::storage::Storage
pub struct ReadyEngineBuilder<
    Order: 'static,
    ExecutionReport: 'static,
    AccountAdjustment: 'static,
    SyncPolicyT,
> where
    SyncPolicyT: sync_policy::SyncPolicy,
    SyncPolicyT::EngineLocking: EnginePolicies<Order, ExecutionReport, AccountAdjustment>,
{
    pre_trade_policies:
        PreTradeVec<SyncPolicyT::EngineLocking, Order, ExecutionReport, AccountAdjustment>,
    storage_builder:
        StorageBuilder<<SyncPolicyT as sync_policy::SyncPolicy>::StorageLockingPolicyFactory>,
    _marker: PhantomData<(Order, ExecutionReport, AccountAdjustment, SyncPolicyT)>,
}

impl<Order, ExecutionReport, AccountAdjustment, SyncPolicyT>
    ReadyEngineBuilder<Order, ExecutionReport, AccountAdjustment, SyncPolicyT>
where
    SyncPolicyT: sync_policy::SyncPolicy,
    SyncPolicyT::EngineLocking: EnginePolicies<Order, ExecutionReport, AccountAdjustment>,
{
    /// Returns the storage builder owned by this engine builder. Pass it (or
    /// a borrowed reference to it) to policy constructors that need internal
    /// storage tables. The factory type is shared with the engine builder's
    /// synchronization policy.
    pub fn storage_builder(
        &self,
    ) -> &StorageBuilder<<SyncPolicyT as sync_policy::SyncPolicy>::StorageLockingPolicyFactory>
    {
        &self.storage_builder
    }
}

impl<Order: 'static, ExecutionReport: 'static, AccountAdjustment: 'static, SyncPolicyT>
    ReadyEngineBuilder<Order, ExecutionReport, AccountAdjustment, SyncPolicyT>
where
    SyncPolicyT: sync_policy::SyncPolicy,
    SyncPolicyT::EngineLocking: EnginePolicies<Order, ExecutionReport, AccountAdjustment>,
{
    /// Registers an additional policy.
    pub fn pre_trade<Policy>(mut self, policy: Policy) -> Self
    where
        Policy: PolicyBox<
            <SyncPolicyT::EngineLocking as EnginePolicies<
                Order,
                ExecutionReport,
                AccountAdjustment,
            >>::PreTrade,
        >,
    {
        self.pre_trade_policies.push(PolicyBox::into_box(policy));
        self
    }

    /// Builds the engine.
    pub fn build(
        self,
    ) -> Result<
        Engine<Order, ExecutionReport, AccountAdjustment, SyncPolicyT::EngineLocking>,
        EngineBuildError,
    > {
        ensure_unique_policy_names(self.pre_trade_policies.iter().map(|p| p.name()))?;
        Ok(Engine {
            inner: SyncPolicyT::EngineLocking::new_strong(EngineInner {
                pre_trade_policies: self.pre_trade_policies,
            }),
        })
    }
}

fn ensure_unique_policy_names<'a>(
    names: impl Iterator<Item = &'a str>,
) -> Result<(), EngineBuildError> {
    let mut unique = HashSet::new();
    for name in names {
        if !unique.insert(name.to_owned()) {
            return Err(EngineBuildError::DuplicatePolicyName {
                name: name.to_owned(),
            });
        }
    }

    Ok(())
}

fn execute_request<
    Order: 'static,
    ExecutionReport: 'static,
    AccountAdjustment: 'static,
    EL: EngineLockingPolicy + EnginePolicies<Order, ExecutionReport, AccountAdjustment>,
>(
    engine: EL::Weak<EngineInner<Order, ExecutionReport, AccountAdjustment, EL>>,
    ctx: PreTradeContext,
    order: Order,
) -> Result<PreTradeReservation, Rejects> {
    let Some(engine_ref) = EL::upgrade(&engine) else {
        return Err(Rejects::new(vec![Reject::new(
            "Engine",
            RejectScope::Order,
            RejectCode::SystemUnavailable,
            "engine is no longer available",
            "request handle outlived engine instance".to_owned(),
        )]));
    };
    let inner: &EngineInner<Order, ExecutionReport, AccountAdjustment, EL> = &engine_ref;

    let mut mutations = Mutations::new();
    let mut lists = Vec::new();
    let mut len = 0;
    for policy in &inner.pre_trade_policies {
        if let Err(rejects) = policy.perform_pre_trade_check(&ctx, &order, &mut mutations) {
            len += rejects.len();
            lists.push(rejects);
        }
    }

    if let Some(rejects) = merge_reject_lists(lists, len) {
        mutations.rollback_all();
        return Err(rejects);
    }

    let reservation_handle = ReservationHandleImpl::new(mutations);
    Ok(PreTradeReservation::from_handle(Box::new(
        reservation_handle,
    )))
}

fn merge_reject_lists(lists: Vec<Rejects>, len: usize) -> Option<Rejects> {
    if len == 0 {
        return None;
    }
    let mut out = Vec::with_capacity(len);
    for rejects in lists {
        out.extend(rejects.into_vec());
    }
    Some(Rejects::new(out))
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use crate::core::{
        ExecutionReportOperation, FinancialImpact, Instrument, OrderOperation,
        WithExecutionReportOperation, WithFinancialImpact, WithOrderOperation,
    };
    use crate::param::{AccountId, Asset, Fee, Pnl, Price, Quantity, Side, TradeAmount, Volume};
    use crate::pretrade::{
        PreTradeContext, PreTradePolicy, Reject, RejectCode, RejectScope, Rejects,
    };
    use crate::{AccountAdjustmentContext, Mutation, Mutations};

    use super::{AccountAdjustmentBatchError, Engine, EngineBuildError, SyncedEngineLocking};

    type TestOrder = WithOrderOperation<()>;
    type TestReport = WithFinancialImpact<WithExecutionReportOperation<()>>;
    type TestAdjustment = MockAdjustment;
    type MutationHook = Rc<dyn Fn(&mut Mutations)>;
    type AdjustmentHook = Box<dyn Fn(&mut Mutations)>;

    #[test]
    fn build_rejects_duplicate_policy_names_across_stages() {
        let result = Engine::<TestOrder, TestReport>::builder()
            .pre_trade(StartPolicyMock::pass("dup"))
            .pre_trade(MainPolicyMock::pass("dup"))
            .build();

        assert!(matches!(
            result,
            Err(EngineBuildError::DuplicatePolicyName { name }) if name == "dup"
        ));
    }

    #[test]
    fn build_rejects_duplicate_policy_names_within_start_stage() {
        let result = Engine::<TestOrder, TestReport>::builder()
            .pre_trade(StartPolicyMock::pass("dup"))
            .pre_trade(StartPolicyMock::pass("dup"))
            .build();

        assert!(matches!(
            result,
            Err(EngineBuildError::DuplicatePolicyName { name }) if name == "dup"
        ));
    }

    #[test]
    fn apply_account_adjustment_passes_adjustment_to_policy_for_single_element() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let engine = Engine::<TestOrder, TestReport, TestAdjustment>::builder()
            .pre_trade(AdjustmentPolicyMock::pass("adj", Rc::clone(&seen)))
            .build()
            .expect("engine must build");

        let batch = [MockAdjustment { id: 7, amount: 120 }];
        let result = engine.apply_account_adjustment(AccountId::from_u64(99224416), &batch);

        assert!(result.is_ok());
        assert_eq!(*seen.borrow(), vec![MockAdjustment { id: 7, amount: 120 }]);
    }

    #[test]
    fn apply_account_adjustment_returns_failing_index() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let engine = Engine::<TestOrder, TestReport, TestAdjustment>::builder()
            .pre_trade(AdjustmentPolicyMock::reject_on_id(
                "adj_reject",
                Rc::clone(&seen),
                1,
            ))
            .build()
            .expect("engine must build");

        let batch = [
            MockAdjustment { id: 0, amount: 10 },
            MockAdjustment { id: 1, amount: 20 },
            MockAdjustment { id: 2, amount: 30 },
        ];
        let result = engine.apply_account_adjustment(AccountId::from_u64(99224416), &batch);

        assert!(matches!(
            result,
            Err(AccountAdjustmentBatchError {
                failed_adjustment_index: 1,
                rejects,
            }) if rejects[0].policy == "adj_reject"
        ));
        assert_eq!(
            *seen.borrow(),
            vec![
                MockAdjustment { id: 0, amount: 10 },
                MockAdjustment { id: 1, amount: 20 },
            ]
        );
    }

    #[test]
    fn apply_account_adjustment_does_not_apply_partial_batch_on_reject() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let mut applied = Vec::new();
        let engine = Engine::<TestOrder, TestReport, TestAdjustment>::builder()
            .pre_trade(AdjustmentPolicyMock::reject_on_id(
                "adj_reject",
                Rc::clone(&seen),
                2,
            ))
            .build()
            .expect("engine must build");

        let batch = [
            MockAdjustment { id: 1, amount: 10 },
            MockAdjustment { id: 2, amount: 20 },
        ];
        let result = engine.apply_account_adjustment(AccountId::from_u64(99224416), &batch);
        if result.is_ok() {
            applied.extend(batch.iter().map(|adjustment| adjustment.id));
        }

        assert!(result.is_err());
        assert!(applied.is_empty());
        assert_eq!(
            *seen.borrow(),
            vec![
                MockAdjustment { id: 1, amount: 10 },
                MockAdjustment { id: 2, amount: 20 },
            ]
        );
    }

    #[test]
    fn apply_account_adjustment_accepts_multi_element_batch_when_all_policies_pass() {
        let first_seen = Rc::new(RefCell::new(Vec::new()));
        let second_seen = Rc::new(RefCell::new(Vec::new()));
        let engine = Engine::<TestOrder, TestReport, TestAdjustment>::builder()
            .pre_trade(AdjustmentPolicyMock::pass("first", Rc::clone(&first_seen)))
            .pre_trade(AdjustmentPolicyMock::pass(
                "second",
                Rc::clone(&second_seen),
            ))
            .build()
            .expect("engine must build");

        let batch = [
            MockAdjustment { id: 10, amount: 1 },
            MockAdjustment { id: 11, amount: 2 },
            MockAdjustment { id: 12, amount: 3 },
        ];

        assert!(engine
            .apply_account_adjustment(AccountId::from_u64(99224416), &batch)
            .is_ok());
        assert_eq!(*first_seen.borrow(), batch);
        assert_eq!(*second_seen.borrow(), batch);
    }

    #[test]
    fn apply_account_adjustment_empty_batch_skips_policy_calls() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let engine = Engine::<TestOrder, TestReport, TestAdjustment>::builder()
            .pre_trade(AdjustmentPolicyMock::pass("adj", Rc::clone(&seen)))
            .build()
            .expect("engine must build");

        assert!(engine
            .apply_account_adjustment(AccountId::from_u64(99224416), &[])
            .is_ok());
        assert!(seen.borrow().is_empty());
    }

    #[test]
    fn apply_account_adjustment_returns_reject_from_second_policy() {
        let first_seen = Rc::new(RefCell::new(Vec::new()));
        let second_seen = Rc::new(RefCell::new(Vec::new()));
        let engine = Engine::<TestOrder, TestReport, TestAdjustment>::builder()
            .pre_trade(AdjustmentPolicyMock::pass("first", Rc::clone(&first_seen)))
            .pre_trade(AdjustmentPolicyMock::reject_on_id(
                "second",
                Rc::clone(&second_seen),
                9,
            ))
            .build()
            .expect("engine must build");

        let batch = [MockAdjustment { id: 9, amount: 99 }];
        let result = engine.apply_account_adjustment(AccountId::from_u64(99224416), &batch);

        assert!(matches!(
            result,
            Err(AccountAdjustmentBatchError {
                failed_adjustment_index: 0,
                rejects,
            }) if rejects[0].policy == "second"
        ));
        assert_eq!(*first_seen.borrow(), batch);
        assert_eq!(*second_seen.borrow(), batch);
    }

    #[test]
    fn apply_account_adjustment_respects_policy_registration_order_for_rejects() {
        let first_seen = Rc::new(RefCell::new(Vec::new()));
        let second_seen = Rc::new(RefCell::new(Vec::new()));
        let engine = Engine::<TestOrder, TestReport, TestAdjustment>::builder()
            .pre_trade(AdjustmentPolicyMock::reject_on_id(
                "first",
                Rc::clone(&first_seen),
                77,
            ))
            .pre_trade(AdjustmentPolicyMock::reject_on_id(
                "second",
                Rc::clone(&second_seen),
                77,
            ))
            .build()
            .expect("engine must build");

        let batch = [MockAdjustment { id: 77, amount: 1 }];
        let result = engine.apply_account_adjustment(AccountId::from_u64(99224416), &batch);

        assert!(matches!(
            result,
            Err(AccountAdjustmentBatchError {
                failed_adjustment_index: 0,
                rejects,
            }) if rejects[0].policy == "first"
        ));
        assert_eq!(*first_seen.borrow(), batch);
        assert!(second_seen.borrow().is_empty());
    }

    #[test]
    fn build_rejects_duplicate_policy_names_between_start_and_account_adjustment() {
        let result = Engine::<TestOrder, TestReport, TestAdjustment>::builder()
            .pre_trade(StartPolicyMock::pass("dup"))
            .pre_trade(AdjustmentPolicyMock::pass(
                "dup",
                Rc::new(RefCell::new(Vec::new())),
            ))
            .build();

        assert!(matches!(
            result,
            Err(EngineBuildError::DuplicatePolicyName { name }) if name == "dup"
        ));
    }

    #[test]
    fn apply_account_adjustment_without_policies_accepts_any_batch() {
        let engine = Engine::<TestOrder, TestReport, TestAdjustment>::builder()
            .build()
            .expect("engine must build");
        let batch = [
            MockAdjustment { id: 1, amount: 11 },
            MockAdjustment { id: 2, amount: 22 },
        ];

        assert!(engine
            .apply_account_adjustment(AccountId::from_u64(99224416), &batch)
            .is_ok());
    }

    #[test]
    fn apply_account_adjustment_commits_mutations_on_success() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let state = Rc::new(RefCell::new(None));
        let engine = Engine::<TestOrder, TestReport, TestAdjustment>::builder()
            .pre_trade(AdjustmentPolicyMock::with_side_effect(
                "adj_mutation",
                Rc::clone(&seen),
                shared_kill_switch_mutation(Rc::clone(&state), "adj_mutation", true, false),
            ))
            .build()
            .expect("engine must build");

        let batch = [MockAdjustment { id: 1, amount: 10 }];
        assert!(engine
            .apply_account_adjustment(AccountId::from_u64(99224416), &batch)
            .is_ok());
        assert_eq!(*state.borrow(), Some(true));
    }

    #[test]
    fn apply_account_adjustment_rolls_back_mutations_on_reject() {
        let first_seen = Rc::new(RefCell::new(Vec::new()));
        let second_seen = Rc::new(RefCell::new(Vec::new()));
        let state = Rc::new(RefCell::new(None));
        let engine = Engine::<TestOrder, TestReport, TestAdjustment>::builder()
            .pre_trade(AdjustmentPolicyMock::with_side_effect(
                "adj_mutation",
                Rc::clone(&first_seen),
                shared_kill_switch_mutation(Rc::clone(&state), "adj_mutation", true, false),
            ))
            .pre_trade(AdjustmentPolicyMock::reject_on_id(
                "adj_rejecter",
                Rc::clone(&second_seen),
                1,
            ))
            .build()
            .expect("engine must build");

        let batch = [MockAdjustment { id: 1, amount: 10 }];
        assert!(engine
            .apply_account_adjustment(AccountId::from_u64(99224416), &batch)
            .is_err());
        assert_eq!(*state.borrow(), Some(false));
    }

    #[test]
    fn apply_account_adjustment_without_mutations_leaves_state_unchanged() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let engine = Engine::<TestOrder, TestReport, TestAdjustment>::builder()
            .pre_trade(AdjustmentPolicyMock::pass("adj", Rc::clone(&seen)))
            .build()
            .expect("engine must build");

        let batch = [MockAdjustment { id: 1, amount: 10 }];
        assert!(engine
            .apply_account_adjustment(AccountId::from_u64(99224416), &batch)
            .is_ok());
    }

    #[test]
    fn engine_builder_with_default_report_type_remains_operational() {
        let engine = Engine::<TestOrder>::builder()
            .build()
            .expect("engine must build with default report type");
        let request = engine
            .start_pre_trade(order_with_settlement("USD"))
            .expect("start stage must pass");
        let mut reservation = request.execute().expect("execute must pass");
        reservation.rollback();

        let post_trade = engine.apply_execution_report(&());
        assert!(!post_trade.kill_switch_triggered);
    }

    #[test]
    fn builder_builds_operational_empty_engine() {
        let mut reservation = Engine::<TestOrder, TestReport>::builder()
            .build()
            .expect("builder must build")
            .start_pre_trade(order_with_settlement("USD"))
            .expect("built engine must allow start stage")
            .execute()
            .expect("built engine must allow execute");
        reservation.rollback();
    }

    #[test]
    fn execute_pre_trade_shortcut_returns_reservation_when_both_stages_pass() {
        let engine = Engine::<TestOrder, TestReport>::builder()
            .pre_trade(StartPolicyMock::pass("start"))
            .pre_trade(MainPolicyMock::pass("main"))
            .build()
            .expect("engine must build");

        let mut reservation = engine
            .execute_pre_trade(order_with_settlement("USD"))
            .expect("shortcut must pass");
        reservation.rollback();
    }

    #[test]
    fn execute_pre_trade_wraps_start_stage_reject_into_single_element_rejects() {
        let engine = Engine::<TestOrder, TestReport>::builder()
            .pre_trade(StartPolicyMock::new(
                "start_reject",
                Rc::new(Cell::new(0)),
                true,
                false,
                None,
                None,
            ))
            .build()
            .expect("engine must build");

        let rejects = match engine.execute_pre_trade(order_with_settlement("USD")) {
            Ok(_) => panic!("start stage must reject"),
            Err(rejects) => rejects,
        };
        assert_eq!(rejects.len(), 1);
        assert_eq!(rejects[0].policy, "start_reject");
        assert_eq!(rejects[0].code, RejectCode::Other);
        assert_eq!(rejects[0].reason, "start reject");
    }

    #[test]
    fn execute_pre_trade_returns_main_stage_rejects_in_original_order() {
        let engine = Engine::<TestOrder, TestReport>::builder()
            .pre_trade(StartPolicyMock::pass("start"))
            .pre_trade(MainPolicyMock::with_mutation_and_optional_reject(
                "main_first",
                "m1",
                true,
                RejectScope::Order,
            ))
            .pre_trade(MainPolicyMock::with_mutation_and_optional_reject(
                "main_second",
                "m2",
                true,
                RejectScope::Account,
            ))
            .build()
            .expect("engine must build");

        let rejects = match engine.execute_pre_trade(order_with_settlement("USD")) {
            Ok(_) => panic!("main stage must reject"),
            Err(rejects) => rejects,
        };
        assert_eq!(rejects.len(), 2);
        assert_eq!(rejects[0].policy, "main_first");
        assert_eq!(rejects[1].policy, "main_second");
        assert_eq!(rejects[0].scope, RejectScope::Order);
        assert_eq!(rejects[1].scope, RejectScope::Account);
    }

    #[test]
    fn execute_pre_trade_commit_applies_mutations_on_success() {
        let state = Rc::new(RefCell::new(None));
        let engine = Engine::<TestOrder, TestReport>::builder()
            .pre_trade(StartPolicyMock::pass("start"))
            .pre_trade(MainPolicyMock::with_custom_mutation_and_optional_reject(
                "main",
                shared_kill_switch_mutation(Rc::clone(&state), "shortcut_commit", true, false),
                false,
                RejectScope::Order,
            ))
            .build()
            .expect("engine must build");

        let mut reservation = engine
            .execute_pre_trade(order_with_settlement("USD"))
            .expect("shortcut must pass");
        reservation.commit();

        assert_eq!(*state.borrow(), Some(true));
    }

    #[test]
    fn execute_pre_trade_reject_does_not_apply_commit_mutations() {
        let state = Rc::new(RefCell::new(None));
        let engine = Engine::<TestOrder, TestReport>::builder()
            .pre_trade(StartPolicyMock::pass("start"))
            .pre_trade(MainPolicyMock::with_custom_mutation_and_optional_reject(
                "rejecting_main",
                shared_kill_switch_mutation(Rc::clone(&state), "shortcut_reject", true, false),
                true,
                RejectScope::Order,
            ))
            .build()
            .expect("engine must build");

        let result = engine.execute_pre_trade(order_with_settlement("USD"));
        assert!(result.is_err(), "shortcut must reject");
        assert_eq!(*state.borrow(), Some(false));
    }

    #[test]
    fn accepts_order_without_operation_fields_when_no_policy_requires_them() {
        let engine = Engine::<(), TestReport>::builder()
            .pre_trade(CoreStartPolicyMock {
                name: "core_start",
                reject: false,
            })
            .pre_trade(CoreMainPolicyMock {
                name: "core_main",
                on_apply: None,
                reject: false,
            })
            .build()
            .expect("engine must build");
        let order = ();
        let mut reservation = engine
            .start_pre_trade(order)
            .expect("start stage must pass")
            .execute()
            .expect("main stage must pass");
        reservation.commit();

        let post_trade = engine.apply_execution_report(&execution_report("USD"));
        assert!(!post_trade.kill_switch_triggered);
    }

    #[test]
    fn order_trade_input_build_rejects_duplicate_policy_names_across_stages() {
        let result = Engine::<(), TestReport>::builder()
            .pre_trade(CoreStartPolicyMock {
                name: "dup",
                reject: false,
            })
            .pre_trade(CoreMainPolicyMock {
                name: "dup",
                on_apply: None,
                reject: false,
            })
            .build();

        assert!(matches!(
            result,
            Err(EngineBuildError::DuplicatePolicyName { name }) if name == "dup"
        ));
    }

    #[test]
    fn order_trade_input_build_rejects_duplicate_policy_names_within_start_stage() {
        let result = Engine::<(), TestReport>::builder()
            .pre_trade(CoreStartPolicyMock {
                name: "dup",
                reject: false,
            })
            .pre_trade(CoreStartPolicyMock {
                name: "dup",
                reject: false,
            })
            .build();

        assert!(matches!(
            result,
            Err(EngineBuildError::DuplicatePolicyName { name }) if name == "dup"
        ));
    }

    #[test]
    fn order_trade_input_start_pre_trade_rejects_before_request_is_created() {
        let engine = Engine::<(), TestReport>::builder()
            .pre_trade(CoreStartPolicyMock {
                name: "core_start_reject",
                reject: true,
            })
            .build()
            .expect("engine must build");
        let order = ();

        let result = engine.start_pre_trade(order);
        let rejects = result.expect_err("start stage must reject");
        assert_eq!(rejects.len(), 1);
        assert_eq!(rejects[0].policy, "core_start_reject");
        assert_eq!(rejects[0].code, RejectCode::Other);

        let post_trade = engine.apply_execution_report(&execution_report("USD"));
        assert!(!post_trade.kill_switch_triggered);
    }

    #[test]
    fn order_core_execute_rejects_and_rolls_back_mutations() {
        let state = Rc::new(RefCell::new(None));
        let engine = Engine::<(), TestReport>::builder()
            .pre_trade(CoreStartPolicyMock {
                name: "core_start",
                reject: false,
            })
            .pre_trade(CoreMainPolicyMock {
                name: "core_main",
                on_apply: Some(Rc::new(shared_kill_switch_mutation(
                    Rc::clone(&state),
                    "core_order_mutation",
                    true,
                    false,
                ))),
                reject: true,
            })
            .build()
            .expect("engine must build");
        let order = ();

        let request = engine
            .start_pre_trade(order)
            .expect("start stage must create request");
        let result = request.execute();
        assert!(result.is_err(), "main stage must reject");
        let rejects = result.err().expect("rejects must be present");
        assert_eq!(rejects.len(), 1);
        assert_eq!(rejects[0].policy, "core_main");
        assert_eq!(rejects[0].code, RejectCode::Other);
        assert_eq!(*state.borrow(), Some(false));

        let post_trade = engine.apply_execution_report(&execution_report("USD"));
        assert!(!post_trade.kill_switch_triggered);
    }

    #[test]
    fn order_core_execute_commit_and_rollback_apply_mutation_callback() {
        let cases = [
            (FinalizeAction::Commit, true),
            (FinalizeAction::Rollback, false),
        ];

        for (action, expected_state) in cases {
            let state = Rc::new(RefCell::new(None));
            let engine = Engine::<(), TestReport>::builder()
                .pre_trade(CoreStartPolicyMock {
                    name: "core_start",
                    reject: false,
                })
                .pre_trade(CoreMainPolicyMock {
                    name: "core_main",
                    on_apply: Some(Rc::new(shared_kill_switch_mutation(
                        Rc::clone(&state),
                        "core_finalize_mutation",
                        true,
                        false,
                    ))),
                    reject: false,
                })
                .build()
                .expect("engine must build");
            let order = ();

            let mut reservation = engine
                .start_pre_trade(order)
                .expect("start stage must create request")
                .execute()
                .expect("main stage must pass");

            match action {
                FinalizeAction::Commit => reservation.commit(),
                FinalizeAction::Rollback => reservation.rollback(),
            }

            assert_eq!(*state.borrow(), Some(expected_state));

            let post_trade = engine.apply_execution_report(&execution_report("USD"));
            assert!(!post_trade.kill_switch_triggered);
        }
    }

    #[test]
    fn start_pre_trade_table_cases_follow_registration_order_and_collect_rejects() {
        struct Case {
            reject_index: Option<usize>,
            expected_calls: [usize; 3],
            expected_main_calls: usize,
            expected_ok: bool,
        }

        let cases = [
            Case {
                reject_index: None,
                expected_calls: [1, 1, 1],
                expected_main_calls: 0,
                expected_ok: true,
            },
            Case {
                reject_index: Some(1),
                expected_calls: [1, 1, 1],
                expected_main_calls: 0,
                expected_ok: false,
            },
        ];

        for case in cases {
            let calls_0 = Rc::new(Cell::new(0));
            let calls_1 = Rc::new(Cell::new(0));
            let calls_2 = Rc::new(Cell::new(0));
            let main_calls = Rc::new(Cell::new(0));

            let start_0 = StartPolicyMock::new("s0", Rc::clone(&calls_0), false, false, None, None);
            let start_1 = StartPolicyMock::new(
                "s1",
                Rc::clone(&calls_1),
                case.reject_index == Some(1),
                false,
                None,
                None,
            );
            let start_2 = StartPolicyMock::new("s2", Rc::clone(&calls_2), false, false, None, None);

            let engine = Engine::<TestOrder, TestReport>::builder()
                .pre_trade(start_0)
                .pre_trade(start_1)
                .pre_trade(start_2)
                .pre_trade(MainPolicyMock::with_calls(
                    "m0",
                    Rc::clone(&main_calls),
                    false,
                    false,
                    None,
                ))
                .build()
                .expect("engine must build");

            let result = engine.start_pre_trade(order_with_settlement("USD"));
            assert_eq!(result.is_ok(), case.expected_ok);
            assert_eq!(calls_0.get(), case.expected_calls[0]);
            assert_eq!(calls_1.get(), case.expected_calls[1]);
            assert_eq!(calls_2.get(), case.expected_calls[2]);
            assert_eq!(main_calls.get(), case.expected_main_calls);
        }
    }

    #[test]
    fn execute_table_cases_cover_success_commit_and_reject_rollback() {
        struct Case {
            fail_first: bool,
            fail_second: bool,
            expected_rejects: usize,
            expected_kill_switch: bool,
        }

        let cases = [
            Case {
                fail_first: false,
                fail_second: false,
                expected_rejects: 0,
                expected_kill_switch: true,
            },
            Case {
                fail_first: true,
                fail_second: true,
                expected_rejects: 2,
                expected_kill_switch: false,
            },
        ];

        for case in cases {
            let state = Rc::new(RefCell::new(None));
            let engine = Engine::<TestOrder, TestReport>::builder()
                .pre_trade(StartPolicyMock::pass("start"))
                .pre_trade(MainPolicyMock::with_custom_mutation_and_optional_reject(
                    "m1_policy",
                    shared_kill_switch_mutation(
                        Rc::clone(&state),
                        "shared_kill_switch",
                        false,
                        false,
                    ),
                    case.fail_first,
                    RejectScope::Order,
                ))
                .pre_trade(MainPolicyMock::with_custom_mutation_and_optional_reject(
                    "m2_policy",
                    shared_kill_switch_mutation(
                        Rc::clone(&state),
                        "shared_kill_switch",
                        true,
                        true,
                    ),
                    case.fail_second,
                    RejectScope::Account,
                ))
                .build()
                .expect("engine must build");

            let request = engine
                .start_pre_trade(order_with_settlement("USD"))
                .expect("start stage must pass");
            let execute_result = request.execute();

            if case.expected_rejects == 0 {
                let mut reservation = execute_result.expect("execute must pass");
                reservation.commit();
            } else {
                assert!(execute_result.is_err(), "execute must reject");
                let rejects = execute_result.err().expect("rejects must be present");
                assert_eq!(rejects.len(), case.expected_rejects);
                assert_eq!(rejects[0].code, RejectCode::Other);
                assert_eq!(rejects[0].scope, RejectScope::Order);
                assert_eq!(rejects[1].code, RejectCode::Other);
                assert_eq!(rejects[1].scope, RejectScope::Account);
            }

            assert_eq!(*state.borrow(), Some(case.expected_kill_switch));
        }
    }

    #[test]
    fn light_stage_changes_are_not_rolled_back_when_execute_rejects() {
        let light_counter = Rc::new(Cell::new(0));
        let engine = Engine::<TestOrder, TestReport>::builder()
            .pre_trade(StartPolicyMock::with_counter(
                "start",
                Rc::clone(&light_counter),
            ))
            .pre_trade(MainPolicyMock::with_mutation_and_optional_reject(
                "rejecting_main",
                "m1",
                true,
                RejectScope::Order,
            ))
            .build()
            .expect("engine must build");

        let request = engine
            .start_pre_trade(order_with_settlement("USD"))
            .expect("start stage must pass");
        assert!(request.execute().is_err(), "execute must reject");

        assert_eq!(light_counter.get(), 1);
    }

    #[test]
    fn reservation_drop_triggers_rollback_in_reverse_order() {
        let state = Rc::new(RefCell::new(None));
        let engine = Engine::<TestOrder, TestReport>::builder()
            .pre_trade(StartPolicyMock::pass("start"))
            .pre_trade(MainPolicyMock::with_custom_mutation_and_optional_reject(
                "m1_policy",
                shared_kill_switch_mutation(Rc::clone(&state), "shared_kill_switch", false, false),
                false,
                RejectScope::Order,
            ))
            .pre_trade(MainPolicyMock::with_custom_mutation_and_optional_reject(
                "m2_policy",
                shared_kill_switch_mutation(Rc::clone(&state), "shared_kill_switch", true, true),
                false,
                RejectScope::Order,
            ))
            .build()
            .expect("engine must build");

        let request = engine
            .start_pre_trade(order_with_settlement("USD"))
            .expect("start stage must pass");
        let reservation = request.execute().expect("execute must pass");
        drop(reservation);

        assert_eq!(*state.borrow(), Some(false));
    }

    #[test]
    fn apply_execution_report_aggregates_kill_switch_triggered() {
        let engine = Engine::<TestOrder, TestReport>::builder()
            .pre_trade(StartPolicyMock::new(
                "start_false",
                Rc::new(Cell::new(0)),
                false,
                false,
                None,
                None,
            ))
            .pre_trade(MainPolicyMock::with_calls(
                "main_true",
                Rc::new(Cell::new(0)),
                false,
                true,
                None,
            ))
            .build()
            .expect("engine must build");

        let result = engine.apply_execution_report(&execution_report("USD"));
        assert!(result.kill_switch_triggered);
    }

    #[test]
    fn request_returns_system_unavailable_when_engine_is_dropped() {
        let request = {
            let engine = Engine::<TestOrder, TestReport>::builder()
                .build()
                .expect("engine must build");
            engine
                .start_pre_trade(order_with_settlement("USD"))
                .expect("start stage must pass")
        };

        let result = request.execute();
        assert!(
            result.is_err(),
            "request must fail when engine is unavailable"
        );
        let rejects = result
            .err()
            .expect("rejects must be present when engine is unavailable");
        assert_eq!(rejects.len(), 1);

        let reject = &rejects[0];
        assert_eq!(reject.policy, "Engine");
        assert_eq!(reject.scope, RejectScope::Order);
        assert_eq!(reject.code, RejectCode::SystemUnavailable);
        assert_eq!(reject.reason, "engine is no longer available");
        assert_eq!(reject.details, "request handle outlived engine instance");
    }

    #[test]
    fn order_core_request_returns_system_unavailable_when_engine_is_dropped() {
        let request = {
            let engine = Engine::<(), TestReport>::builder()
                .build()
                .expect("engine must build");
            engine.start_pre_trade(()).expect("start stage must pass")
        };

        let result = request.execute();
        assert!(
            result.is_err(),
            "request must fail when engine is unavailable"
        );
        let rejects = result.err().expect("rejects must be present");
        assert_eq!(rejects.len(), 1);
        assert_eq!(rejects[0].policy, "Engine");
        assert_eq!(rejects[0].scope, RejectScope::Order);
        assert_eq!(rejects[0].code, RejectCode::SystemUnavailable);
    }

    #[test]
    fn reservation_mutation_callback_is_noop_when_engine_is_dropped() {
        let state = Rc::new(RefCell::new(None));
        let mut reservation = {
            let engine = Engine::<TestOrder, TestReport>::builder()
                .pre_trade(StartPolicyMock::pass("start"))
                .pre_trade(MainPolicyMock::with_custom_mutation_and_optional_reject(
                    "main",
                    shared_kill_switch_mutation(
                        Rc::clone(&state),
                        "shared_kill_switch",
                        false,
                        false,
                    ),
                    false,
                    RejectScope::Order,
                ))
                .build()
                .expect("engine must build");

            let request = engine
                .start_pre_trade(order_with_settlement("USD"))
                .expect("start stage must pass");
            request.execute().expect("main stage must pass")
        };

        reservation.commit();
    }

    #[test]
    fn order_core_reservation_mutation_callback_is_noop_when_engine_is_dropped() {
        let state = Rc::new(RefCell::new(None));
        let mut reservation = {
            let engine = Engine::<(), TestReport>::builder()
                .pre_trade(CoreStartPolicyMock {
                    name: "core_start",
                    reject: false,
                })
                .pre_trade(CoreMainPolicyMock {
                    name: "core_main",
                    on_apply: Some(Rc::new(shared_kill_switch_mutation(
                        Rc::clone(&state),
                        "core_drop_mutation",
                        true,
                        false,
                    ))),
                    reject: false,
                })
                .build()
                .expect("engine must build");

            engine
                .start_pre_trade(())
                .expect("start stage must pass")
                .execute()
                .expect("main stage must pass")
        };

        reservation.commit();
    }

    #[test]
    fn build_error_display_is_stable() {
        let err = EngineBuildError::DuplicatePolicyName {
            name: "dup".to_string(),
        };
        assert_eq!(err.to_string(), "duplicate policy name: dup");
    }

    #[test]
    fn account_adjustment_batch_error_display_is_stable() {
        let err = AccountAdjustmentBatchError {
            failed_adjustment_index: 2,
            rejects: Rejects::from(Reject::new(
                "adj_policy",
                RejectScope::Order,
                RejectCode::Other,
                "account adjustment rejected",
                "mock account adjustment policy rejected the adjustment",
            )),
        };
        assert_eq!(
            err.to_string(),
            "account adjustment batch rejected at index 2: [adj_policy] account adjustment rejected: mock account adjustment policy rejected the adjustment"
        );
    }

    #[test]
    fn main_stage_observes_settlement_assets_independently() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let engine = Engine::<TestOrder, TestReport>::builder()
            .pre_trade(StartPolicyMock::pass("start"))
            .pre_trade(MainPolicyMock::with_calls(
                "collector",
                Rc::new(Cell::new(0)),
                false,
                false,
                Some(Rc::clone(&seen)),
            ))
            .build()
            .expect("engine must build");

        let request_usd = engine
            .start_pre_trade(order_with_settlement("USD"))
            .expect("USD order must pass start stage");
        let mut reservation_usd = request_usd.execute().expect("USD order must pass");
        reservation_usd.commit();

        let request_eur = engine
            .start_pre_trade(order_with_settlement("EUR"))
            .expect("EUR order must pass start stage");
        let mut reservation_eur = request_eur.execute().expect("EUR order must pass");
        reservation_eur.commit();

        let seen = seen.borrow();
        assert_eq!(seen.len(), 2);
        assert_eq!(
            seen[0],
            Asset::new("USD").expect("asset code must be valid")
        );
        assert_eq!(
            seen[1],
            Asset::new("EUR").expect("asset code must be valid")
        );
    }

    #[test]
    fn reset_like_start_policy_state_allows_trading_to_resume() {
        let blocked = Rc::new(Cell::new(true));
        let engine = Engine::<TestOrder, TestReport>::builder()
            .pre_trade(StartPolicyMock::with_block_flag(
                "toggle",
                Rc::clone(&blocked),
            ))
            .build()
            .expect("engine must build");

        let first = engine.start_pre_trade(order_with_settlement("USD"));
        assert!(first.is_err());

        blocked.set(false);

        let second = engine.start_pre_trade(order_with_settlement("USD"));
        assert!(second.is_ok());
    }

    #[test]
    fn tagged_build_rejects_duplicate_policy_names_across_stages() {
        let journal = Rc::new(RefCell::new(Vec::new()));
        let seen_orders = Rc::new(RefCell::new(Vec::new()));
        let seen_reports = Rc::new(RefCell::new(Vec::new()));

        let result = Engine::<TaggedOrder, TaggedReport>::builder()
            .pre_trade(CaptureTaggedStartPolicy::new(
                "dup",
                Rc::clone(&journal),
                Rc::clone(&seen_orders),
                Rc::clone(&seen_reports),
            ))
            .pre_trade(CaptureTaggedMainPolicy::new(
                "dup",
                Rc::clone(&journal),
                Rc::clone(&seen_orders),
                Rc::clone(&seen_reports),
            ))
            .build();

        assert!(matches!(
            result,
            Err(EngineBuildError::DuplicatePolicyName { name }) if name == "dup"
        ));
    }

    #[test]
    fn tagged_build_rejects_duplicate_policy_names_within_start_stage() {
        let journal = Rc::new(RefCell::new(Vec::new()));
        let seen_orders = Rc::new(RefCell::new(Vec::new()));
        let seen_reports = Rc::new(RefCell::new(Vec::new()));

        let result = Engine::<TaggedOrder, TaggedReport>::builder()
            .pre_trade(CaptureTaggedStartPolicy::new(
                "dup",
                Rc::clone(&journal),
                Rc::clone(&seen_orders),
                Rc::clone(&seen_reports),
            ))
            .pre_trade(SequenceFenceStartPolicy::new("dup", Rc::clone(&journal)))
            .build();

        assert!(matches!(
            result,
            Err(EngineBuildError::DuplicatePolicyName { name }) if name == "dup"
        ));
    }

    #[test]
    fn tagged_start_pre_trade_rejects_before_request_is_created() {
        let engine = Engine::<TaggedOrder, TaggedReport>::builder()
            .pre_trade(RejectTaggedStartPolicyMock {
                name: "tagged_start_reject",
            })
            .build()
            .expect("engine must build");

        let result = engine.start_pre_trade(tagged_order("ord-reject", "AAPL", "1", "10"));
        let rejects = result.expect_err("start stage must reject");
        assert_eq!(rejects.len(), 1);
        assert_eq!(rejects[0].policy, "tagged_start_reject");
        assert_eq!(rejects[0].code, RejectCode::Other);

        let post_trade =
            engine.apply_execution_report(&tagged_execution_report("rep-reject", "AAPL", "1", "1"));
        assert!(!post_trade.kill_switch_triggered);
    }

    #[test]
    fn tagged_request_returns_system_unavailable_when_engine_is_dropped() {
        let request = {
            let engine = Engine::<TaggedOrder, TaggedReport>::builder()
                .build()
                .expect("engine must build");
            engine
                .start_pre_trade(tagged_order("ord-dropped", "AAPL", "1", "10"))
                .expect("start stage must pass")
        };

        let result = request.execute();
        assert!(
            result.is_err(),
            "request must fail when engine is unavailable"
        );
        let rejects = result.err().expect("rejects must be present");
        assert_eq!(rejects.len(), 1);
        assert_eq!(rejects[0].policy, "Engine");
        assert_eq!(rejects[0].scope, RejectScope::Order);
        assert_eq!(rejects[0].code, RejectCode::SystemUnavailable);
    }

    #[test]
    fn tagged_execute_rejects_and_rolls_back_mutations() {
        let state = Rc::new(RefCell::new(None));
        let engine = Engine::<TaggedOrder, TaggedReport>::builder()
            .pre_trade(TaggedMutationPolicyMock {
                name: "tagged_main",
                on_apply: Some(Rc::new(shared_kill_switch_mutation(
                    Rc::clone(&state),
                    "tagged_reject_mutation",
                    true,
                    false,
                ))),
                reject: true,
            })
            .build()
            .expect("engine must build");

        let request = engine
            .start_pre_trade(tagged_order("ord-tagged-reject", "AAPL", "2", "11"))
            .expect("start stage must create request");
        let result = request.execute();
        assert!(result.is_err(), "main stage must reject");
        let rejects = result.err().expect("rejects must be present");
        assert_eq!(rejects.len(), 1);
        assert_eq!(rejects[0].policy, "tagged_main");
        assert_eq!(rejects[0].code, RejectCode::Other);
        assert_eq!(*state.borrow(), Some(false));

        let post_trade = engine.apply_execution_report(&tagged_execution_report(
            "rep-tagged-reject",
            "AAPL",
            "2",
            "1",
        ));
        assert!(!post_trade.kill_switch_triggered);
    }

    #[test]
    fn tagged_execute_commit_and_rollback_apply_mutation_callback() {
        let cases = [
            (FinalizeAction::Commit, true),
            (FinalizeAction::Rollback, false),
        ];

        for (action, expected_state) in cases {
            let state = Rc::new(RefCell::new(None));
            let engine = Engine::<TaggedOrder, TaggedReport>::builder()
                .pre_trade(TaggedMutationPolicyMock {
                    name: "tagged_main",
                    on_apply: Some(Rc::new(shared_kill_switch_mutation(
                        Rc::clone(&state),
                        "tagged_finalize_mutation",
                        true,
                        false,
                    ))),
                    reject: false,
                })
                .build()
                .expect("engine must build");

            let mut reservation = engine
                .start_pre_trade(tagged_order("ord-tagged-finalize", "MSFT", "3", "12"))
                .expect("start stage must create request")
                .execute()
                .expect("main stage must pass");

            match action {
                FinalizeAction::Commit => reservation.commit(),
                FinalizeAction::Rollback => reservation.rollback(),
            }

            assert_eq!(*state.borrow(), Some(expected_state));

            let post_trade = engine.apply_execution_report(&tagged_execution_report(
                "rep-tagged-finalize",
                "MSFT",
                "3",
                "1",
            ));
            assert!(!post_trade.kill_switch_triggered);
        }
    }

    #[test]
    fn tagged_reservation_mutation_callback_is_noop_when_engine_is_dropped() {
        let state = Rc::new(RefCell::new(None));
        let mut reservation = {
            let engine = Engine::<TaggedOrder, TaggedReport>::builder()
                .pre_trade(TaggedMutationPolicyMock {
                    name: "tagged_main",
                    on_apply: Some(Rc::new(shared_kill_switch_mutation(
                        Rc::clone(&state),
                        "tagged_drop_mutation",
                        true,
                        false,
                    ))),
                    reject: false,
                })
                .build()
                .expect("engine must build");

            engine
                .start_pre_trade(tagged_order("ord-tagged-drop", "AAPL", "1", "10"))
                .expect("start stage must pass")
                .execute()
                .expect("main stage must pass")
        };

        reservation.commit();
    }

    #[test]
    fn interleaved_requests_and_reports_preserve_original_tags_across_all_policies() {
        struct Case {
            execute_order: [usize; 3],
            finalize_actions: [FinalizeAction; 3],
            report_order: [usize; 3],
        }

        let cases = [
            Case {
                execute_order: [2, 0, 1],
                finalize_actions: [
                    FinalizeAction::Rollback,
                    FinalizeAction::Commit,
                    FinalizeAction::Rollback,
                ],
                report_order: [1, 2, 0],
            },
            Case {
                execute_order: [1, 2, 0],
                finalize_actions: [
                    FinalizeAction::Commit,
                    FinalizeAction::Rollback,
                    FinalizeAction::Commit,
                ],
                report_order: [2, 0, 1],
            },
            Case {
                execute_order: [0, 2, 1],
                finalize_actions: [
                    FinalizeAction::Commit,
                    FinalizeAction::Commit,
                    FinalizeAction::Rollback,
                ],
                report_order: [0, 1, 2],
            },
        ];

        for case in cases {
            let journal = Rc::new(RefCell::new(Vec::new()));
            let start_seen_orders = Rc::new(RefCell::new(Vec::new()));
            let start_seen_reports = Rc::new(RefCell::new(Vec::new()));
            let main_seen_orders = Rc::new(RefCell::new(Vec::new()));
            let main_seen_reports = Rc::new(RefCell::new(Vec::new()));

            let engine = Engine::<TaggedOrder, TaggedReport>::builder()
                .pre_trade(CaptureTaggedStartPolicy::new(
                    "capture_start",
                    Rc::clone(&journal),
                    Rc::clone(&start_seen_orders),
                    Rc::clone(&start_seen_reports),
                ))
                .pre_trade(SequenceFenceStartPolicy::new(
                    "sequence_start",
                    Rc::clone(&journal),
                ))
                .pre_trade(CaptureTaggedMainPolicy::new(
                    "capture_main",
                    Rc::clone(&journal),
                    Rc::clone(&main_seen_orders),
                    Rc::clone(&main_seen_reports),
                ))
                .pre_trade(SequenceFenceMainPolicy::new(
                    "sequence_main",
                    Rc::clone(&journal),
                ))
                .build()
                .expect("engine must build");

            let orders = [
                tagged_order("ord-a", "AAPL", "10", "25"),
                tagged_order("ord-b", "MSFT", "11", "26"),
                tagged_order("ord-c", "TSLA", "12", "27"),
            ];
            let reports = [
                tagged_execution_report("rep-a", "AAPL", "5", "1"),
                tagged_execution_report("rep-b", "MSFT", "6", "1"),
                tagged_execution_report("rep-c", "TSLA", "7", "1"),
            ];

            let mut requests: Vec<_> = orders
                .iter()
                .cloned()
                .map(|order| {
                    Some(
                        engine
                            .start_pre_trade(order)
                            .expect("start stage must pass for tagged order"),
                    )
                })
                .collect();

            for (request_index, action) in
                case.execute_order.iter().zip(case.finalize_actions.iter())
            {
                let request = requests[*request_index]
                    .take()
                    .expect("request must be available exactly once");
                let mut reservation = request
                    .execute()
                    .expect("main stage must pass for tagged order");

                match action {
                    FinalizeAction::Commit => reservation.commit(),
                    FinalizeAction::Rollback => reservation.rollback(),
                }
                journal.borrow_mut().push(format!(
                    "finalize:{}:{}",
                    action.as_str(),
                    orders[*request_index].tag
                ));
            }

            for report_index in case.report_order {
                let post_trade = engine.apply_execution_report(&reports[report_index]);
                assert!(!post_trade.kill_switch_triggered);
            }

            assert_eq!(*start_seen_orders.borrow(), vec!["ord-a", "ord-b", "ord-c"]);
            assert_eq!(
                *main_seen_orders.borrow(),
                case.execute_order
                    .iter()
                    .map(|index| orders[*index].tag)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                *start_seen_reports.borrow(),
                case.report_order
                    .iter()
                    .map(|index| reports[*index].tag)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                *main_seen_reports.borrow(),
                case.report_order
                    .iter()
                    .map(|index| reports[*index].tag)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                *journal.borrow(),
                expected_interleaving_journal(
                    &case.execute_order,
                    &case.finalize_actions,
                    &case.report_order,
                )
            );
        }
    }

    #[test]
    fn start_pre_trade_allows_extreme_price_without_notional_precompute() {
        let engine = Engine::<TestOrder, TestReport>::builder()
            .build()
            .expect("engine must build");
        let order = WithOrderOperation {
            inner: (),
            operation: OrderOperation {
                instrument: Instrument::new(
                    Asset::new("AAPL").expect("asset code must be valid"),
                    Asset::new("USD").expect("asset code must be valid"),
                ),
                account_id: AccountId::from_u64(99224416),
                side: Side::Buy,
                trade_amount: TradeAmount::Quantity(
                    Quantity::from_str("1").expect("quantity must be valid"),
                ),
                price: Some(Price::from_str("100").expect("price must be valid")),
            },
        };

        let mut reservation = engine
            .start_pre_trade(order)
            .expect("request must be created without notional precompute")
            .execute()
            .expect("execute without policies must pass");
        reservation.rollback();
    }

    #[test]
    fn sell_order_can_reserve_notional_without_engine_notional_cache() {
        let usd = Asset::new("USD").expect("asset code must be valid");
        let reserved_amount = Volume::from_str("20000").expect("volume must be valid");
        let reserved_notional = Rc::new(RefCell::new(None));
        let engine = Engine::<TestOrder, TestReport>::builder()
            .pre_trade(ReserveNotionalPolicy {
                settlement: usd.clone(),
                amount: reserved_amount,
                reserved_notional: Rc::clone(&reserved_notional),
            })
            .build()
            .expect("engine must build");

        let order = WithOrderOperation {
            inner: (),
            operation: OrderOperation {
                instrument: Instrument::new(
                    Asset::new("AAPL").expect("asset code must be valid"),
                    usd.clone(),
                ),
                account_id: AccountId::from_u64(99224416),
                side: Side::Sell,
                trade_amount: TradeAmount::Quantity(
                    Quantity::from_str("100").expect("quantity must be valid"),
                ),
                price: Some(Price::from_str("200").expect("price must be valid")),
            },
        };

        let mut reservation = engine
            .start_pre_trade(order)
            .expect("sell order must pass start stage")
            .execute()
            .expect("sell order must pass execute");
        reservation.commit();
        assert_eq!(*reserved_notional.borrow(), Some(reserved_amount));

        let post_trade = engine.apply_execution_report(&execution_report("USD"));
        assert!(!post_trade.kill_switch_triggered);
    }

    #[test]
    fn sell_order_reservation_rollback_resets_reserved_notional_to_zero() {
        let usd = Asset::new("USD").expect("asset code must be valid");
        let reserved_amount = Volume::from_str("20000").expect("volume must be valid");
        let reserved_notional = Rc::new(RefCell::new(None));
        let engine = Engine::<TestOrder, TestReport>::builder()
            .pre_trade(ReserveNotionalPolicy {
                settlement: usd.clone(),
                amount: reserved_amount,
                reserved_notional: Rc::clone(&reserved_notional),
            })
            .build()
            .expect("engine must build");

        let order = WithOrderOperation {
            inner: (),
            operation: OrderOperation {
                instrument: Instrument::new(
                    Asset::new("AAPL").expect("asset code must be valid"),
                    usd,
                ),
                account_id: AccountId::from_u64(99224416),
                side: Side::Sell,
                trade_amount: TradeAmount::Quantity(
                    Quantity::from_str("100").expect("quantity must be valid"),
                ),
                price: Some(Price::from_str("200").expect("price must be valid")),
            },
        };

        let mut reservation = engine
            .start_pre_trade(order)
            .expect("sell order must pass start stage")
            .execute()
            .expect("sell order must pass execute");
        reservation.rollback();

        assert_eq!(*reserved_notional.borrow(), Some(Volume::ZERO));
    }

    #[test]
    #[should_panic(expected = "quantity-based order expected")]
    fn reserve_notional_policy_panics_when_volume_order_is_passed() {
        let policy = ReserveNotionalPolicy {
            settlement: Asset::new("USD").expect("asset code must be valid"),
            amount: Volume::from_str("100").expect("volume must be valid"),
            reserved_notional: Rc::new(RefCell::new(None)),
        };
        let order = WithOrderOperation {
            inner: (),
            operation: OrderOperation {
                instrument: Instrument::new(
                    Asset::new("AAPL").expect("asset code must be valid"),
                    Asset::new("USD").expect("asset code must be valid"),
                ),
                account_id: AccountId::from_u64(99224416),
                side: Side::Sell,
                trade_amount: TradeAmount::Volume(
                    Volume::from_str("100").expect("volume must be valid"),
                ),
                price: Some(Price::from_str("200").expect("price must be valid")),
            },
        };
        let mut mutations = Mutations::default();
        let _ = <ReserveNotionalPolicy as PreTradePolicy<TestOrder, TestReport>>::perform_pre_trade_check(
            &policy,
            &PreTradeContext::new(),
            &order,
            &mut mutations,
        );
    }

    #[test]
    #[should_panic(expected = "assertion `left == right` failed")]
    fn reserve_notional_policy_panics_for_volume_based_order() {
        let policy = ReserveNotionalPolicy {
            settlement: Asset::new("USD").expect("asset code must be valid"),
            amount: Volume::from_str("100").expect("volume must be valid"),
            reserved_notional: Rc::new(RefCell::new(None)),
        };
        let order = WithOrderOperation {
            inner: (),
            operation: OrderOperation {
                instrument: Instrument::new(
                    Asset::new("AAPL").expect("asset code must be valid"),
                    Asset::new("USD").expect("asset code must be valid"),
                ),
                account_id: AccountId::from_u64(99224416),
                side: Side::Sell,
                trade_amount: TradeAmount::Quantity(
                    Quantity::from_str("100").expect("quantity must be valid"),
                ),
                price: Some(Price::from_str("200").expect("price must be valid")),
            },
        };
        let mut mutations = Mutations::default();
        let _ = <ReserveNotionalPolicy as PreTradePolicy<TestOrder, TestReport>>::perform_pre_trade_check(
            &policy,
            &PreTradeContext::new(),
            &order,
            &mut mutations,
        );
    }

    fn order_with_settlement(settlement: &str) -> TestOrder {
        WithOrderOperation {
            inner: (),
            operation: OrderOperation {
                instrument: Instrument::new(
                    Asset::new("AAPL").expect("asset code must be valid"),
                    Asset::new(settlement).expect("asset code must be valid"),
                ),
                account_id: AccountId::from_u64(99224416),
                side: Side::Buy,
                trade_amount: TradeAmount::Quantity(
                    Quantity::from_str("1").expect("quantity must be valid"),
                ),
                price: Some(Price::from_str("100").expect("price must be valid")),
            },
        }
    }

    fn execution_report(settlement: &str) -> TestReport {
        WithFinancialImpact {
            inner: WithExecutionReportOperation {
                inner: (),
                operation: ExecutionReportOperation {
                    instrument: Instrument::new(
                        Asset::new("AAPL").expect("asset code must be valid"),
                        Asset::new(settlement).expect("asset code must be valid"),
                    ),
                    account_id: AccountId::from_u64(99224416),
                    side: Side::Buy,
                },
            },
            financial_impact: FinancialImpact {
                pnl: Pnl::from_str("-10").expect("pnl must be valid"),
                fee: Fee::from_str("1").expect("fee must be valid"),
            },
        }
    }

    #[derive(Clone, Copy)]
    enum FinalizeAction {
        Commit,
        Rollback,
    }

    impl FinalizeAction {
        fn as_str(self) -> &'static str {
            match self {
                Self::Commit => "commit",
                Self::Rollback => "rollback",
            }
        }
    }

    #[derive(Clone)]
    struct TaggedOrder {
        tag: &'static str,
    }

    #[derive(Clone)]
    struct TaggedReport {
        tag: &'static str,
    }

    struct CaptureTaggedStartPolicy {
        name: &'static str,
        journal: Rc<RefCell<Vec<String>>>,
        seen_orders: Rc<RefCell<Vec<&'static str>>>,
        seen_reports: Rc<RefCell<Vec<&'static str>>>,
    }

    impl CaptureTaggedStartPolicy {
        fn new(
            name: &'static str,
            journal: Rc<RefCell<Vec<String>>>,
            seen_orders: Rc<RefCell<Vec<&'static str>>>,
            seen_reports: Rc<RefCell<Vec<&'static str>>>,
        ) -> Self {
            Self {
                name,
                journal,
                seen_orders,
                seen_reports,
            }
        }
    }

    impl<AccountAdjustment> PreTradePolicy<TaggedOrder, TaggedReport, AccountAdjustment>
        for CaptureTaggedStartPolicy
    {
        fn name(&self) -> &str {
            self.name
        }

        fn check_pre_trade_start(
            &self,
            _ctx: &PreTradeContext,
            order: &TaggedOrder,
        ) -> Result<(), Rejects> {
            self.seen_orders.borrow_mut().push(order.tag);
            self.journal
                .borrow_mut()
                .push(format!("start:{}:{}", self.name, order.tag));
            Ok(())
        }

        fn apply_execution_report(&self, report: &TaggedReport) -> bool {
            self.seen_reports.borrow_mut().push(report.tag);
            self.journal
                .borrow_mut()
                .push(format!("report-start:{}:{}", self.name, report.tag));
            false
        }
    }

    struct SequenceFenceStartPolicy {
        name: &'static str,
        journal: Rc<RefCell<Vec<String>>>,
    }

    impl SequenceFenceStartPolicy {
        fn new(name: &'static str, journal: Rc<RefCell<Vec<String>>>) -> Self {
            Self { name, journal }
        }
    }

    impl<AccountAdjustment> PreTradePolicy<TaggedOrder, TaggedReport, AccountAdjustment>
        for SequenceFenceStartPolicy
    {
        fn name(&self) -> &str {
            self.name
        }

        fn check_pre_trade_start(
            &self,
            _ctx: &PreTradeContext,
            order: &TaggedOrder,
        ) -> Result<(), Rejects> {
            self.journal
                .borrow_mut()
                .push(format!("start:{}:{}", self.name, order.tag));
            Ok(())
        }

        fn apply_execution_report(&self, report: &TaggedReport) -> bool {
            self.journal
                .borrow_mut()
                .push(format!("report-start:{}:{}", self.name, report.tag));
            false
        }
    }

    struct CaptureTaggedMainPolicy {
        name: &'static str,
        journal: Rc<RefCell<Vec<String>>>,
        seen_orders: Rc<RefCell<Vec<&'static str>>>,
        seen_reports: Rc<RefCell<Vec<&'static str>>>,
    }

    impl CaptureTaggedMainPolicy {
        fn new(
            name: &'static str,
            journal: Rc<RefCell<Vec<String>>>,
            seen_orders: Rc<RefCell<Vec<&'static str>>>,
            seen_reports: Rc<RefCell<Vec<&'static str>>>,
        ) -> Self {
            Self {
                name,
                journal,
                seen_orders,
                seen_reports,
            }
        }
    }

    impl<AccountAdjustment> PreTradePolicy<TaggedOrder, TaggedReport, AccountAdjustment>
        for CaptureTaggedMainPolicy
    {
        fn name(&self) -> &str {
            self.name
        }

        fn perform_pre_trade_check(
            &self,
            _ctx: &PreTradeContext,
            order: &TaggedOrder,
            _mutations: &mut Mutations,
        ) -> Result<(), Rejects> {
            self.seen_orders.borrow_mut().push(order.tag);
            self.journal
                .borrow_mut()
                .push(format!("execute:{}:{}", self.name, order.tag));
            Ok(())
        }

        fn apply_execution_report(&self, report: &TaggedReport) -> bool {
            self.seen_reports.borrow_mut().push(report.tag);
            self.journal
                .borrow_mut()
                .push(format!("report-main:{}:{}", self.name, report.tag));
            false
        }
    }

    struct SequenceFenceMainPolicy {
        name: &'static str,
        journal: Rc<RefCell<Vec<String>>>,
    }

    impl SequenceFenceMainPolicy {
        fn new(name: &'static str, journal: Rc<RefCell<Vec<String>>>) -> Self {
            Self { name, journal }
        }
    }

    impl<AccountAdjustment> PreTradePolicy<TaggedOrder, TaggedReport, AccountAdjustment>
        for SequenceFenceMainPolicy
    {
        fn name(&self) -> &str {
            self.name
        }

        fn perform_pre_trade_check(
            &self,
            _ctx: &PreTradeContext,
            order: &TaggedOrder,
            _mutations: &mut Mutations,
        ) -> Result<(), Rejects> {
            self.journal
                .borrow_mut()
                .push(format!("execute:{}:{}", self.name, order.tag));
            Ok(())
        }

        fn apply_execution_report(&self, report: &TaggedReport) -> bool {
            self.journal
                .borrow_mut()
                .push(format!("report-main:{}:{}", self.name, report.tag));
            false
        }
    }

    struct RejectTaggedStartPolicyMock {
        name: &'static str,
    }

    impl<AccountAdjustment> PreTradePolicy<TaggedOrder, TaggedReport, AccountAdjustment>
        for RejectTaggedStartPolicyMock
    {
        fn name(&self) -> &str {
            self.name
        }

        fn check_pre_trade_start(
            &self,
            _ctx: &PreTradeContext,
            _order: &TaggedOrder,
        ) -> Result<(), Rejects> {
            Err(Rejects::from(Reject::new(
                self.name,
                RejectScope::Order,
                RejectCode::Other,
                "tagged start reject",
                "tagged start policy rejected the order",
            )))
        }

        fn apply_execution_report(&self, _report: &TaggedReport) -> bool {
            false
        }
    }

    struct TaggedMutationPolicyMock {
        name: &'static str,
        on_apply: Option<MutationHook>,
        reject: bool,
    }

    impl<AccountAdjustment> PreTradePolicy<TaggedOrder, TaggedReport, AccountAdjustment>
        for TaggedMutationPolicyMock
    {
        fn name(&self) -> &str {
            self.name
        }

        fn perform_pre_trade_check(
            &self,
            _ctx: &PreTradeContext,
            _order: &TaggedOrder,
            mutations: &mut Mutations,
        ) -> Result<(), Rejects> {
            if let Some(on_apply) = &self.on_apply {
                on_apply(mutations);
            }

            if self.reject {
                return Err(Rejects::from(Reject::new(
                    self.name,
                    RejectScope::Order,
                    RejectCode::Other,
                    "tagged main reject",
                    "tagged mutation policy rejected the order",
                )));
            }
            Ok(())
        }

        fn apply_execution_report(&self, _report: &TaggedReport) -> bool {
            false
        }
    }

    struct CoreStartPolicyMock {
        name: &'static str,
        reject: bool,
    }

    impl<AccountAdjustment> PreTradePolicy<(), TestReport, AccountAdjustment> for CoreStartPolicyMock {
        fn name(&self) -> &str {
            self.name
        }

        fn check_pre_trade_start(
            &self,
            _ctx: &PreTradeContext,
            _order: &(),
        ) -> Result<(), Rejects> {
            if self.reject {
                return Err(Rejects::from(Reject::new(
                    self.name,
                    RejectScope::Order,
                    RejectCode::Other,
                    "core start reject",
                    "order core start policy rejected the order",
                )));
            }
            Ok(())
        }

        fn apply_execution_report(&self, _report: &TestReport) -> bool {
            false
        }
    }

    struct CoreMainPolicyMock {
        name: &'static str,
        on_apply: Option<MutationHook>,
        reject: bool,
    }

    impl<AccountAdjustment> PreTradePolicy<(), TestReport, AccountAdjustment> for CoreMainPolicyMock {
        fn name(&self) -> &str {
            self.name
        }

        fn perform_pre_trade_check(
            &self,
            _ctx: &PreTradeContext,
            _order: &(),
            mutations: &mut Mutations,
        ) -> Result<(), Rejects> {
            if let Some(on_apply) = &self.on_apply {
                on_apply(mutations);
            }

            if self.reject {
                return Err(Rejects::from(Reject::new(
                    self.name,
                    RejectScope::Order,
                    RejectCode::Other,
                    "core main reject",
                    "order core main policy rejected the order",
                )));
            }
            Ok(())
        }

        fn apply_execution_report(&self, _report: &TestReport) -> bool {
            false
        }
    }

    fn tagged_order(
        tag: &'static str,
        _underlying: &'static str,
        _quantity: &'static str,
        _price: &'static str,
    ) -> TaggedOrder {
        TaggedOrder { tag }
    }

    fn tagged_execution_report(
        tag: &'static str,
        _underlying: &'static str,
        _pnl: &'static str,
        _fee: &'static str,
    ) -> TaggedReport {
        TaggedReport { tag }
    }

    fn expected_interleaving_journal(
        execute_order: &[usize; 3],
        finalize_actions: &[FinalizeAction; 3],
        report_order: &[usize; 3],
    ) -> Vec<String> {
        let order_tags = ["ord-a", "ord-b", "ord-c"];
        let report_tags = ["rep-a", "rep-b", "rep-c"];
        let mut expected = Vec::new();

        for order_tag in order_tags {
            expected.push(format!("start:capture_start:{order_tag}"));
            expected.push(format!("start:sequence_start:{order_tag}"));
        }

        for (request_index, action) in execute_order.iter().zip(finalize_actions.iter()) {
            let order_tag = order_tags[*request_index];
            expected.push(format!("execute:capture_main:{order_tag}"));
            expected.push(format!("execute:sequence_main:{order_tag}"));
            expected.push(format!("finalize:{}:{order_tag}", action.as_str()));
        }

        for report_index in report_order {
            let report_tag = report_tags[*report_index];
            expected.push(format!("report-start:capture_start:{report_tag}"));
            expected.push(format!("report-start:sequence_start:{report_tag}"));
            expected.push(format!("report-main:capture_main:{report_tag}"));
            expected.push(format!("report-main:sequence_main:{report_tag}"));
        }

        expected
    }

    struct StartPolicyMock {
        name: &'static str,
        calls: Rc<Cell<usize>>,
        reject: bool,
        post_trade_trigger: bool,
        light_counter: Option<Rc<Cell<usize>>>,
        block_flag: Option<Rc<Cell<bool>>>,
    }

    impl StartPolicyMock {
        fn new(
            name: &'static str,
            calls: Rc<Cell<usize>>,
            reject: bool,
            post_trade_trigger: bool,
            light_counter: Option<Rc<Cell<usize>>>,
            block_flag: Option<Rc<Cell<bool>>>,
        ) -> Self {
            Self {
                name,
                calls,
                reject,
                post_trade_trigger,
                light_counter,
                block_flag,
            }
        }

        fn pass(name: &'static str) -> Self {
            Self::new(name, Rc::new(Cell::new(0)), false, false, None, None)
        }

        fn with_counter(name: &'static str, counter: Rc<Cell<usize>>) -> Self {
            Self::new(
                name,
                Rc::new(Cell::new(0)),
                false,
                false,
                Some(counter),
                None,
            )
        }

        fn with_block_flag(name: &'static str, block_flag: Rc<Cell<bool>>) -> Self {
            Self::new(
                name,
                Rc::new(Cell::new(0)),
                false,
                false,
                None,
                Some(block_flag),
            )
        }
    }

    impl<AccountAdjustment> PreTradePolicy<TestOrder, TestReport, AccountAdjustment>
        for StartPolicyMock
    {
        fn name(&self) -> &str {
            self.name
        }

        fn check_pre_trade_start(
            &self,
            _ctx: &PreTradeContext,
            _order: &TestOrder,
        ) -> Result<(), Rejects> {
            self.calls.set(self.calls.get() + 1);
            if let Some(counter) = &self.light_counter {
                counter.set(counter.get() + 1);
            }
            if let Some(block_flag) = &self.block_flag {
                if block_flag.get() {
                    return Err(Rejects::from(Reject::new(
                        self.name,
                        RejectScope::Account,
                        RejectCode::PnlKillSwitchTriggered,
                        "pnl kill switch triggered",
                        "mock policy blocked the account",
                    )));
                }
            }
            if self.reject {
                return Err(Rejects::from(Reject::new(
                    self.name,
                    RejectScope::Order,
                    RejectCode::Other,
                    "start reject",
                    "mock start policy rejected the order",
                )));
            }
            Ok(())
        }

        fn apply_execution_report(&self, _report: &TestReport) -> bool {
            self.post_trade_trigger
        }
    }

    struct MainPolicyMock {
        name: &'static str,
        calls: Rc<Cell<usize>>,
        reject: bool,
        reject_scope: RejectScope,
        on_apply: Option<MutationHook>,
        post_trade_trigger: bool,
        seen_settlement: Option<Rc<RefCell<Vec<Asset>>>>,
    }

    impl MainPolicyMock {
        fn pass(name: &'static str) -> Self {
            Self {
                name,
                calls: Rc::new(Cell::new(0)),
                reject: false,
                reject_scope: RejectScope::Order,
                on_apply: None,
                post_trade_trigger: false,
                seen_settlement: None,
            }
        }

        fn with_calls(
            name: &'static str,
            calls: Rc<Cell<usize>>,
            reject: bool,
            post_trade_trigger: bool,
            seen_settlement: Option<Rc<RefCell<Vec<Asset>>>>,
        ) -> Self {
            Self {
                name,
                calls,
                reject,
                reject_scope: RejectScope::Order,
                on_apply: None,
                post_trade_trigger,
                seen_settlement,
            }
        }

        fn with_mutation_and_optional_reject(
            name: &'static str,
            mutation_id: &'static str,
            reject: bool,
            reject_scope: RejectScope,
        ) -> Self {
            let state = Rc::new(RefCell::new(None));
            Self::with_custom_mutation_and_optional_reject(
                name,
                shared_kill_switch_mutation(state, mutation_id, true, false),
                reject,
                reject_scope,
            )
        }

        fn with_custom_mutation_and_optional_reject(
            name: &'static str,
            on_apply: impl Fn(&mut Mutations) + 'static,
            reject: bool,
            reject_scope: RejectScope,
        ) -> Self {
            Self {
                name,
                calls: Rc::new(Cell::new(0)),
                reject,
                reject_scope,
                on_apply: Some(Rc::new(on_apply)),
                post_trade_trigger: false,
                seen_settlement: None,
            }
        }
    }

    impl<AccountAdjustment> PreTradePolicy<TestOrder, TestReport, AccountAdjustment>
        for MainPolicyMock
    {
        fn name(&self) -> &str {
            self.name
        }

        fn perform_pre_trade_check(
            &self,
            _ctx: &PreTradeContext,
            order: &TestOrder,
            mutations: &mut Mutations,
        ) -> Result<(), Rejects> {
            self.calls.set(self.calls.get() + 1);
            if let Some(seen_settlement) = &self.seen_settlement {
                seen_settlement
                    .borrow_mut()
                    .push(order.operation.instrument.settlement_asset().clone());
            }

            if let Some(on_apply) = &self.on_apply {
                on_apply(mutations);
            }

            if self.reject {
                return Err(Rejects::from(Reject::new(
                    self.name,
                    self.reject_scope.clone(),
                    RejectCode::Other,
                    "main reject",
                    "mock main-stage policy rejected the order",
                )));
            }
            Ok(())
        }

        fn apply_execution_report(&self, _report: &TestReport) -> bool {
            self.post_trade_trigger
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct MockAdjustment {
        id: u32,
        amount: i64,
    }

    struct AdjustmentPolicyMock {
        name: &'static str,
        seen: Rc<RefCell<Vec<MockAdjustment>>>,
        reject_on_id: Option<u32>,
        on_apply: Option<AdjustmentHook>,
    }

    impl AdjustmentPolicyMock {
        fn pass(name: &'static str, seen: Rc<RefCell<Vec<MockAdjustment>>>) -> Self {
            Self {
                name,
                seen,
                reject_on_id: None,
                on_apply: None,
            }
        }

        fn reject_on_id(
            name: &'static str,
            seen: Rc<RefCell<Vec<MockAdjustment>>>,
            reject_on_id: u32,
        ) -> Self {
            Self {
                name,
                seen,
                reject_on_id: Some(reject_on_id),
                on_apply: None,
            }
        }

        fn with_side_effect(
            name: &'static str,
            seen: Rc<RefCell<Vec<MockAdjustment>>>,
            on_apply: impl Fn(&mut Mutations) + 'static,
        ) -> Self {
            Self {
                name,
                seen,
                reject_on_id: None,
                on_apply: Some(Box::new(on_apply)),
            }
        }
    }

    impl<Order, ExecutionReport> PreTradePolicy<Order, ExecutionReport, MockAdjustment>
        for AdjustmentPolicyMock
    {
        fn name(&self) -> &str {
            self.name
        }

        fn apply_account_adjustment(
            &self,
            _ctx: &AccountAdjustmentContext,
            _account_id: AccountId,
            adjustment: &MockAdjustment,
            mutations: &mut Mutations,
        ) -> Result<(), Rejects> {
            self.seen.borrow_mut().push(*adjustment);
            if let Some(ref on_apply) = self.on_apply {
                on_apply(mutations);
            }
            if self.reject_on_id == Some(adjustment.id) {
                return Err(Rejects::from(Reject::new(
                    self.name,
                    RejectScope::Order,
                    RejectCode::Other,
                    "account adjustment rejected",
                    "mock account adjustment policy rejected the adjustment",
                )));
            }
            Ok(())
        }
    }

    struct ReserveNotionalPolicy {
        settlement: Asset,
        amount: Volume,
        reserved_notional: Rc<RefCell<Option<Volume>>>,
    }

    fn shared_kill_switch_mutation(
        state: Rc<RefCell<Option<bool>>>,
        _id: &'static str,
        commit_value: bool,
        rollback_value: bool,
    ) -> impl Fn(&mut Mutations) {
        move |mutations: &mut Mutations| {
            let c = Rc::clone(&state);
            let r = Rc::clone(&state);
            mutations.push(Mutation::new(
                move || {
                    *c.borrow_mut() = Some(commit_value);
                },
                move || {
                    *r.borrow_mut() = Some(rollback_value);
                },
            ));
        }
    }

    impl<AccountAdjustment> PreTradePolicy<TestOrder, TestReport, AccountAdjustment>
        for ReserveNotionalPolicy
    {
        fn name(&self) -> &str {
            "ReserveNotionalPolicy"
        }

        fn perform_pre_trade_check(
            &self,
            _ctx: &PreTradeContext,
            order: &TestOrder,
            mutations: &mut Mutations,
        ) -> Result<(), Rejects> {
            use crate::core::{HasOrderPrice, HasTradeAmount};
            assert_eq!(order.operation.side, Side::Sell);
            assert_eq!(
                order.operation.instrument.settlement_asset(),
                &self.settlement
            );
            let calculated_amount = order
                .price()
                .expect("price must be present")
                .expect("price value must be Some")
                .calculate_volume(
                    match order.trade_amount().expect("trade_amount must be present") {
                        TradeAmount::Quantity(value) => value,
                        TradeAmount::Volume(_) => panic!("quantity-based order expected"),
                    },
                )
                .expect("volume must be calculable");
            assert_eq!(calculated_amount, self.amount);
            let commit_store = Rc::clone(&self.reserved_notional);
            let rollback_store = Rc::clone(&self.reserved_notional);
            let commit_amount = self.amount;
            mutations.push(Mutation::new(
                move || {
                    *commit_store.borrow_mut() = Some(commit_amount);
                },
                move || {
                    *rollback_store.borrow_mut() = Some(Volume::ZERO);
                },
            ));
            Ok(())
        }

        fn apply_execution_report(&self, _report: &TestReport) -> bool {
            false
        }
    }

    // ── Send/Sync type assertions ─────────────────────────────────────────────

    #[test]
    fn synced_engine_with_full_storage_is_send_and_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        type SyncedEngineWithFull = Engine<OrderOperation, (), (), SyncedEngineLocking>;
        assert_send::<SyncedEngineWithFull>();
        assert_sync::<SyncedEngineWithFull>();
    }

    /// Confirms that `LocalEngineLocking`-flavored builders accept `!Send` policies.
    /// The `!Send + !Sync` property of `Engine<..., LocalEngineLocking>` is enforced by
    /// `Rc` auto-deriving `!Send + !Sync`; see the compile_fail doctest on
    /// `LocalEngineLocking` for explicit proof.
    #[test]
    fn local_engine_accepts_not_send_policies() {
        // StartPolicyMock contains Rc<...> (!Send). Confirms the local builder
        // compiles with !Send policies.
        let _engine = Engine::<TestOrder, TestReport>::builder()
            .pre_trade(StartPolicyMock::pass("local_policy"))
            .build()
            .expect("engine with !Send policy must build in local mode");
    }
}
