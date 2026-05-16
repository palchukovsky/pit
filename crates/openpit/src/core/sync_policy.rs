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

//! Engine-level synchronization configuration.

/// Consolidates the synchronization settings for an [`Engine`](crate::Engine)
/// instance.
///
/// A `SyncPolicy` is a purely type-level construct — typically a zero-sized
/// struct — that bundles all synchronization configuration in one place. The
/// engine builder accepts it through
/// [`EngineBuilder::sync`](crate::EngineBuilder::sync) and carries
/// it through the builder type chain into [`SyncedEngineBuilder`] and
/// [`ReadyEngineBuilder`], giving trading policies (which receive a
/// `&`[`StorageBuilder`]) a uniform surface to create their internal data
/// tables regardless of the deployment's threading model.
///
/// Currently the main dimensions of synchronization are storage locking
/// (`StorageLockingPolicyFactory`) and engine-handle locking
/// (`EngineLocking`). The trait is designed as an extension point: future
/// releases may add dimensions such as thread-pool affinity,
/// contention-monitoring hooks, or back-pressure strategies. Account sync
/// produces a `Send + !Sync` engine handle: ownership may move across
/// threads sequentially, but concurrent sharing of the same handle is not
/// supported.
///
/// # Implementing a custom policy
///
/// Define a zero-sized struct and implement this trait:
///
/// ```rust
/// use openpit::storage::FullLocking;
/// use openpit::{SyncPolicy, SyncedEngineLocking};
///
/// struct MyFullySyncPolicy;
///
/// impl SyncPolicy for MyFullySyncPolicy {
///     type StorageLockingPolicyFactory = FullLocking;
///     type EngineLocking = SyncedEngineLocking;
///     fn create_locking_factory(&self) -> FullLocking { FullLocking }
/// }
/// ```
///
/// Pass it to the engine builder via turbofish:
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use openpit::{Engine, SyncPolicy};
/// use openpit::storage::FullLocking;
///
/// struct MyFullySyncPolicy;
/// impl SyncPolicy for MyFullySyncPolicy {
///     type StorageLockingPolicyFactory = FullLocking;
///     type EngineLocking = openpit::SyncedEngineLocking;
///     fn create_locking_factory(&self) -> FullLocking { FullLocking }
/// }
///
/// type MyOrder = openpit::WithOrderOperation<()>;
///
/// use openpit::pretrade::policies::{RateLimit, RateLimitBrokerBarrier, RateLimitPolicy};
///
/// let builder = Engine::<MyOrder>::builder().sync(MyFullySyncPolicy);
/// let policy = RateLimitPolicy::new(
///     Some(RateLimitBrokerBarrier {
///         limit: RateLimit {
///             max_orders: 100,
///             window: std::time::Duration::from_secs(1),
///         },
///     }),
///     [],
///     [],
///     [],
///     builder.storage_builder(),
/// )?;
/// let _engine = builder
///     .pre_trade(policy)
///     .build()?;
/// # Ok(())
/// # }
/// ```
///
/// # Customizing storage locking
///
/// `StorageLockingPolicyFactory` controls how every [`Storage`] created by
/// trading policies registered with this engine acquires and releases locks.
/// The three built-in choices are:
///
/// * [`FullLocking`] — two independent reader-writer locks (one for the key
///   index, one for all values). All data accesses are fully synchronized.
/// * [`IndexLocking`] — one reader-writer lock guards key insertions and
///   removals; per-value access is the caller's responsibility.
/// * [`NoLocking`] — no synchronization at all. The resulting storages are
///   `!Send + !Sync`; suitable for single-threaded embeddings.
///
/// If none of these fits, you can implement a fully custom locking strategy:
///
/// 1. **Implement [`storage::LockingPolicy`]** — the `unsafe` trait that
///    provides four lock-acquisition methods:
///    - `read_index` / `write_index` — shared and exclusive access to the
///      key index (controls concurrent insertions, removals, and lookups).
///    - `read_values` / `write_values` — shared and exclusive access to all
///      values (controls concurrent reads and mutations). Both methods receive
///      the entry's key as a generic argument, allowing per-key granularity
///      in custom implementations (the built-in policies ignore it).
///
/// 2. **Implement [`storage::LockingPolicyFactory`]** — produces a fresh
///    `LockingPolicy` instance for each storage. Each storage built by
///    [`StorageBuilder::create`] gets its own factory-produced policy, so
///    locks are per-storage.
///
/// 3. **Set `StorageLockingPolicyFactory`** to your factory in a `SyncPolicy`
///    implementation and pass it to the engine builder.
///
/// The safety contract for `LockingPolicy` is documented in the
/// [`storage`](crate::storage) module.
///
/// [`Storage`]: crate::storage::Storage
/// [`StorageBuilder`]: crate::StorageBuilder
/// [`StorageBuilder::create`]: crate::StorageBuilder::create
/// [`storage::LockingPolicy`]: crate::storage::LockingPolicy
/// [`storage::LockingPolicyFactory`]: crate::storage::LockingPolicyFactory
/// [`FullLocking`]: crate::storage::FullLocking
/// [`IndexLocking`]: crate::storage::IndexLocking
/// [`NoLocking`]: crate::storage::NoLocking
/// [`SyncedEngineBuilder`]: crate::SyncedEngineBuilder
/// [`ReadyEngineBuilder`]: crate::ReadyEngineBuilder
pub trait SyncPolicy {
    /// Factory type used to create [`storage::LockingPolicy`] instances for
    /// [`Storage`] tables owned by policies registered with this engine.
    ///
    /// This factory is handed to every [`StorageBuilder`] that trading
    /// policies obtain through the engine builder. Each call to
    /// [`StorageBuilder::create`] produces a storage whose synchronization
    /// regime is determined by this factory.
    ///
    /// [`storage::LockingPolicy`]: crate::storage::LockingPolicy
    /// [`Storage`]: crate::storage::Storage
    /// [`StorageBuilder`]: crate::StorageBuilder
    /// [`StorageBuilder::create`]: crate::StorageBuilder::create
    type StorageLockingPolicyFactory: crate::storage::LockingPolicyFactory;

    /// Smart-pointer pair used by the engine handle for its internal state.
    ///
    /// Controls whether the [`Engine`](crate::Engine) produced by the builder
    /// is `Send + Sync`, `Send + !Sync`, or `!Send + !Sync`:
    ///
    /// - [`LocalEngineLocking`](crate::LocalEngineLocking) (`Rc`/`Weak`) —
    ///   the engine handle is `!Send + !Sync`. Zero atomic-refcount overhead.
    ///   Used by [`LocalSyncPolicy`].
    ///
    /// - [`SyncedEngineLocking`](crate::SyncedEngineLocking) (`Arc`/`Weak`) —
    ///   the engine handle inherits `Send + Sync` from its inner state.
    ///   Used by [`FullSyncPolicy`].
    ///
    /// - [`SequentialEngineLocking`](crate::SequentialEngineLocking) —
    ///   the engine handle is `Send + !Sync`. Ownership may move between OS
    ///   threads sequentially, with one active public-method call per handle
    ///   at a time. Used by [`AccountSyncPolicy`].
    type EngineLocking: crate::EngineLockingPolicy;

    /// Creates the locking-policy factory for this sync policy.
    ///
    /// Called once by [`EngineBuilder::sync`](crate::EngineBuilder::sync)
    /// to initialise the [`StorageBuilder`](crate::StorageBuilder) that is
    /// handed to registered trading policies.
    fn create_locking_factory(&self) -> Self::StorageLockingPolicyFactory;
}

// ─── Built-in concrete sync policies ─────────────────────────────────────────

/// Full thread-safety synchronization policy for [`Engine`](crate::Engine)
/// storages.
///
/// Concrete [`SyncPolicy`] used by
/// [`EngineBuilder::full_sync`](crate::EngineBuilder::full_sync).
/// Storage tables created by registered trading policies use
/// [`FullLocking`](crate::storage::FullLocking): two independent
/// reader-writer locks (one for the key index, one for all values) ensure
/// every data access is fully synchronized.
///
/// Most callers reach this policy through the builder shortcut. Name the
/// type directly only when implementing a custom [`SyncPolicy`] that
/// delegates to it, or when calling
/// [`EngineBuilder::sync`](crate::EngineBuilder::sync) explicitly.
pub struct FullSyncPolicy;

/// Single-thread (no-sync) synchronization policy for
/// [`Engine`](crate::Engine) storages.
///
/// Concrete [`SyncPolicy`] used by
/// [`EngineBuilder::no_sync`](crate::EngineBuilder::no_sync).
/// Storage tables created by registered trading policies use
/// [`NoLocking`](crate::storage::NoLocking): no synchronization primitives
/// are allocated. The resulting storages are `!Send + !Sync`; this policy is
/// for single-threaded embeddings where synchronization overhead must be
/// zero.
///
/// Most callers reach this policy through the builder shortcut. Name the
/// type directly only when implementing a custom [`SyncPolicy`] that
/// delegates to it, or when calling
/// [`EngineBuilder::sync`](crate::EngineBuilder::sync) explicitly.
pub struct LocalSyncPolicy;

/// Account-keyed synchronization policy for [`Engine`](crate::Engine)
/// storages and sequential cross-thread engine handles.
///
/// Concrete [`SyncPolicy`] used by
/// [`EngineBuilder::account_sync`](crate::EngineBuilder::account_sync).
/// Storage tables created by registered trading policies use
/// [`IndexLocking<AccountKeyConstraint>`](crate::storage::IndexLocking):
/// one reader-writer lock guards key insertions and removals, and the
/// engine builder enforces at compile time that every storage key
/// identifies an account (see [`AccountKey`](crate::AccountKey)).
///
/// The resulting engine handle is `Send + !Sync`: callers may move ownership
/// between OS threads sequentially, but concurrent invocation on the same
/// handle is not supported. This policy fits account-sharded embeddings where
/// the caller pins each account to a single processing chain.
///
/// Most callers reach this policy through the builder shortcut. Name the
/// type directly only when implementing a custom [`SyncPolicy`] that
/// delegates to it, or when calling
/// [`EngineBuilder::sync`](crate::EngineBuilder::sync) explicitly.
pub struct AccountSyncPolicy;

impl SyncPolicy for FullSyncPolicy {
    type StorageLockingPolicyFactory = crate::storage::FullLocking;
    type EngineLocking = crate::SyncedEngineLocking;

    fn create_locking_factory(&self) -> Self::StorageLockingPolicyFactory {
        Default::default()
    }
}

impl SyncPolicy for LocalSyncPolicy {
    type StorageLockingPolicyFactory = crate::storage::NoLocking;
    type EngineLocking = crate::LocalEngineLocking;

    fn create_locking_factory(&self) -> Self::StorageLockingPolicyFactory {
        Default::default()
    }
}

impl SyncPolicy for AccountSyncPolicy {
    type StorageLockingPolicyFactory = crate::storage::IndexLocking<crate::AccountKeyConstraint>;
    type EngineLocking = crate::SequentialEngineLocking;

    fn create_locking_factory(&self) -> Self::StorageLockingPolicyFactory {
        Default::default()
    }
}
