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

//! Threading-regime abstraction for [`Engine`](crate::Engine) handles.

use std::ops::Deref;

/// Smart-pointer pair used by an [`Engine`](crate::Engine) and its deferred handles.
/// Selected at engine-build time; orthogonal to the storage
/// [`LockingPolicy`](crate::storage::LockingPolicy).
///
/// Three built-in impls cover the canonical cases:
///
/// * [`LocalEngineLocking`] — engine handle is `!Send + !Sync`; the caller
///   keeps every reference on the OS thread that created the engine. No
///   runtime synchronization overhead.
/// * [`SyncedEngineLocking`] — engine handle inherits `Send + Sync` from its
///   inner state. With `FullLocking` storage the engine is thread-safe and
///   supports concurrent invocation from multiple threads.
/// * [`SequentialEngineLocking`] — engine handle is `Send + !Sync`; ownership
///   may move between OS threads sequentially, but concurrent invocation on
///   the same handle is not supported.
///
/// The [`EngineBuilder`](crate::EngineBuilder) chain maps sync policies to
/// engine-locking flavors automatically:
///
/// - [`no_sync`](crate::EngineBuilder::no_sync) →
///   [`LocalEngineLocking`]
/// - [`full_sync`](crate::EngineBuilder::full_sync) →
///   [`SyncedEngineLocking`]
/// - [`account_sync`](crate::EngineBuilder::account_sync) →
///   [`SequentialEngineLocking`]
///
/// [`LocalEngineLocking`] is the **default** for the `EngineBuilder` chain:
/// existing single-threaded code that names `Engine<MyOrder>` continues to
/// compile unchanged.
///
/// # Safety
///
/// Implementations must guarantee that the strong and weak types form
/// a sound strong-weak pair: while at least one strong handle is alive,
/// `upgrade` on a weak handle returns `Some(...)`; when all strong
/// handles are dropped, `upgrade` returns `None`. The built-in
/// implementations satisfy this.
///
/// # Stability and extension
///
/// The trait is sealed. Three implementations ship today:
/// [`LocalEngineLocking`] (single-thread, zero runtime overhead),
/// [`SyncedEngineLocking`] (multi-thread, `Send + Sync` engine handle for
/// concurrent invocation), and [`SequentialEngineLocking`] (multi-thread
/// sequential, `Send + !Sync` engine handle for account-sharded workloads).
/// Custom implementations are not supported: the trait is sealed.
pub trait EngineLockingPolicy: crate::__private::Sealed + 'static {
    /// Strong reference type. Must implement `Clone` and `Deref<Target = T>`.
    type Strong<T: 'static>: Clone + Deref<Target = T>;
    /// Weak reference type corresponding to [`Strong`](Self::Strong).
    ///
    /// Must be `'static` so it can be captured by `'static` closures (e.g. deferred
    /// pre-trade request handles that outlive the scope where the engine was borrowed).
    type Weak<T: 'static>: 'static;

    /// Wraps `inner` in a new strong reference.
    fn new_strong<T: 'static>(inner: T) -> Self::Strong<T>;
    /// Creates a weak reference from the given strong reference.
    fn downgrade<T: 'static>(s: &Self::Strong<T>) -> Self::Weak<T>;
    /// Attempts to upgrade a weak reference to a strong reference.
    ///
    /// Returns `Some` if at least one strong reference is still alive,
    /// `None` if all strong references have been dropped.
    fn upgrade<T: 'static>(w: &Self::Weak<T>) -> Option<Self::Strong<T>>;
}

/// Engine handle is `!Send + !Sync`; the caller keeps every reference on the
/// OS thread that created the engine. No runtime synchronization overhead.
///
/// This is the **default** engine-locking policy. It is selected by:
///
/// - [`EngineBuilder::no_sync`](crate::EngineBuilder::no_sync)
///
/// Registered policies are required to be `'static` only; non-Send policy
/// state (e.g. non-atomic counters in tests) is fully supported.
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use openpit::{Engine, LocalEngine};
/// use openpit::pretrade::policies::OrderValidationPolicy;
/// use openpit::OrderOperation;
///
/// let engine: LocalEngine<OrderOperation> = Engine::<OrderOperation>::builder()
///     .no_sync()
///     .pre_trade(OrderValidationPolicy::new())
///     .build()?;
/// # Ok(())
/// # }
/// ```
///
/// `LocalEngine<T>` is `!Send + !Sync`; attempting to send it across threads does not
/// compile:
///
/// ```compile_fail
/// use openpit::{LocalEngine, OrderOperation};
/// use openpit::pretrade::policies::OrderValidationPolicy;
///
/// fn require_send<T: Send>(_: T) {}
///
/// let engine = LocalEngine::<OrderOperation>::builder()
///     .no_sync()
///     .pre_trade(OrderValidationPolicy::new())
///     .build()
///     .unwrap();
/// require_send(engine); // compile error: LocalEngine is !Send
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalEngineLocking;

impl crate::__private::Sealed for LocalEngineLocking {}

impl EngineLockingPolicy for LocalEngineLocking {
    type Strong<T: 'static> = std::rc::Rc<T>;
    type Weak<T: 'static> = std::rc::Weak<T>;

    fn new_strong<T: 'static>(inner: T) -> Self::Strong<T> {
        std::rc::Rc::new(inner)
    }

    fn downgrade<T: 'static>(s: &Self::Strong<T>) -> Self::Weak<T> {
        std::rc::Rc::downgrade(s)
    }

    fn upgrade<T: 'static>(w: &Self::Weak<T>) -> Option<Self::Strong<T>> {
        w.upgrade()
    }
}

/// Engine handle inherits `Send + Sync` from its inner state. With
/// `FullLocking` storage (selected by
/// [`EngineBuilder::full_sync`](crate::EngineBuilder::full_sync))
/// the engine's internal state is `Send + Sync`, so the resulting engine
/// supports concurrent invocation from multiple threads and can be wrapped
/// in `Arc<Engine>` or moved between threads.
///
/// This policy is selected by:
///
/// - [`EngineBuilder::full_sync`](crate::EngineBuilder::full_sync)
///
/// Registered policies must be `Send + Sync + 'static`. The builder enforces
/// this at compile time for the `SyncedEngineLocking`-flavored builder chain.
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use openpit::{Engine, SyncedEngine};
/// use openpit::pretrade::policies::OrderValidationPolicy;
/// use openpit::OrderOperation;
/// use std::sync::Arc;
///
/// let engine: Arc<SyncedEngine<OrderOperation>> = Arc::new(
///     Engine::<OrderOperation>::builder()
///         .full_sync()
///         .pre_trade(OrderValidationPolicy::new())
///         .build()?,
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub struct SyncedEngineLocking;

impl crate::__private::Sealed for SyncedEngineLocking {}

impl EngineLockingPolicy for SyncedEngineLocking {
    type Strong<T: 'static> = std::sync::Arc<T>;
    type Weak<T: 'static> = std::sync::Weak<T>;

    fn new_strong<T: 'static>(inner: T) -> Self::Strong<T> {
        std::sync::Arc::new(inner)
    }

    fn downgrade<T: 'static>(s: &Self::Strong<T>) -> Self::Weak<T> {
        std::sync::Arc::downgrade(s)
    }

    fn upgrade<T: 'static>(w: &Self::Weak<T>) -> Option<Self::Strong<T>> {
        w.upgrade()
    }
}

/// Engine handle handed out for account-sharded sequential cross-thread
/// invocation.
///
/// `Send`: ownership of the engine handle may move between OS threads
/// sequentially, with the caller serialising per-handle invocation (one
/// active public-method call per handle at a time). Concurrent invocation
/// on the same handle is forbidden by contract and not supported at the
/// type level (the handle is `!Sync`).
///
/// The same wrapper is used by `pit-interop`'s `EngineLocking` under
/// `SyncMode::Account`; this type lives in `openpit` so pure Rust SDK
/// clients reach the same contract through `account_sync()`.
///
/// # Pure Rust vs binding-layer contract
///
/// Unlike the binding-layer `openpit_interop::EngineLocking` under
/// `SyncMode::Account`, this pure-Rust handle does **not** allow concurrent
/// invocation from multiple threads even when calls are partitioned by
/// account. The `!Sync` bound enforces this at compile time. Rust SDK clients
/// who need per-account concurrency must shard ownership: place each
/// `Arc<Mutex<Engine<...>>>` (or equivalent) behind a per-account lock, or
/// move handles between threads sequentially with explicit serialisation.
pub struct SequentialEngineHandle<T: ?Sized>(
    std::sync::Arc<T>,
    std::marker::PhantomData<std::cell::Cell<()>>,
);

impl<T: ?Sized> Clone for SequentialEngineHandle<T> {
    fn clone(&self) -> Self {
        Self(std::sync::Arc::clone(&self.0), std::marker::PhantomData)
    }
}

impl<T: ?Sized> Deref for SequentialEngineHandle<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

// SAFETY:
// Sending an `Arc<T>` between threads requires `T: Send + Sync` for
// `std::sync::Arc<T>: Send`. We claim `Send` with only `T: Send`
// because the SDK's account-sync threading contract requires the
// caller to serialise per-handle invocation: ownership of the handle
// may move between OS threads sequentially, but only one thread
// observes `&EngineInner` at any moment. The `Arc` refcount is
// thread-safe; the inner state is observed by at most one thread at
// a time per the contract.
//
// The marker field deliberately keeps this handle `!Sync`: concurrent
// shared access is not supported under account-sharded synchronization,
// and the type system reflects that. Callers attempting
// `Arc<Engine<..., SequentialEngineLocking>>` and concurrent invocation
// receive a compile error.
unsafe impl<T: ?Sized + Send> Send for SequentialEngineHandle<T> {}

/// Weak counterpart of [`SequentialEngineHandle`].
pub struct SequentialEngineHandleWeak<T: ?Sized>(
    std::sync::Weak<T>,
    std::marker::PhantomData<std::cell::Cell<()>>,
);

impl<T: ?Sized> Clone for SequentialEngineHandleWeak<T> {
    fn clone(&self) -> Self {
        Self(std::sync::Weak::clone(&self.0), std::marker::PhantomData)
    }
}

// SAFETY: same sequential ownership-transfer contract as
// `SequentialEngineHandle` above. The weak handle owns no engine state;
// it only carries the thread-safe weak reference count.
unsafe impl<T: ?Sized + Send> Send for SequentialEngineHandleWeak<T> {}

/// Account-sharded engine-locking policy.
///
/// Engine handle is `Send + !Sync`. Selected by
/// [`EngineBuilder::account_sync`](crate::EngineBuilder::account_sync).
///
/// # Pure Rust vs binding-layer contract
///
/// This policy is `!Sync`: the Rust type system forbids concurrent invocation
/// on the same handle regardless of how calls are partitioned. This differs
/// from `openpit_interop::EngineLocking` under `SyncMode::Account`, which allows
/// concurrent invocation when the caller guarantees per-account serialisation.
/// Rust SDK clients who need per-account concurrency must shard `Engine`
/// ownership — one handle (or one `Arc<Mutex<Engine<...>>>`) per account shard.
#[derive(Debug, Default, Clone, Copy)]
pub struct SequentialEngineLocking;

impl crate::__private::Sealed for SequentialEngineLocking {}

impl EngineLockingPolicy for SequentialEngineLocking {
    type Strong<T: 'static> = SequentialEngineHandle<T>;
    type Weak<T: 'static> = SequentialEngineHandleWeak<T>;

    fn new_strong<T: 'static>(inner: T) -> Self::Strong<T> {
        SequentialEngineHandle(std::sync::Arc::new(inner), std::marker::PhantomData)
    }

    fn downgrade<T: 'static>(s: &Self::Strong<T>) -> Self::Weak<T> {
        SequentialEngineHandleWeak(std::sync::Arc::downgrade(&s.0), std::marker::PhantomData)
    }

    fn upgrade<T: 'static>(w: &Self::Weak<T>) -> Option<Self::Strong<T>> {
        w.0.upgrade()
            .map(|inner| SequentialEngineHandle(inner, std::marker::PhantomData))
    }
}
