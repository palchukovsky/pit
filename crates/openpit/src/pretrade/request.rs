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

use super::reject::Reject;
use super::reservation::Reservation;

/// Opaque capability object representing a deferred pre-trade execution stage.
///
/// Created by [`crate::Engine::start_pre_trade`] after start-stage policies pass.
/// Holds a single-use capability: once [`Request::execute`] is called, the
/// object is consumed and cannot be reused.
///
/// The request does not expose the underlying order or notional to the caller;
/// those values are visible only to the engine and the policies.
pub struct Request {
    inner: Box<dyn RequestHandle>,
}

/// Internal capability interface used by [`Request`].
pub(crate) trait RequestHandle {
    /// Executes the deferred pre-trade stage.
    fn execute(self: Box<Self>) -> Result<Reservation, Vec<Reject>>;
}

impl Request {
    /// Executes deferred pre-trade checks.
    ///
    /// The call is single-use by type semantics because `self` is consumed.
    ///
    /// # Examples
    ///
    /// ```
    /// use openpit::param::{Asset, Price, Quantity, Side};
    /// use openpit::core::Instrument;
    /// use openpit::{Engine, Order};
    ///
    /// let engine = Engine::builder().build().expect("valid config");
    /// let order = Order {
    ///     instrument: Instrument::new(
    ///         Asset::new("AAPL").expect("asset code must be valid"),
    ///         Asset::new("USD").expect("asset code must be valid"),
    ///     ),
    ///     side: Side::Buy,
    ///     quantity: Quantity::from_str("10").expect("valid"),
    ///     price: Price::from_str("185").expect("valid"),
    /// };
    /// let request = engine.start_pre_trade(order).expect("start stage must pass");
    /// let reservation = request.execute().expect("main stage must pass");
    /// reservation.commit();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `Vec<Reject>` when any main-stage policy rejects the order.
    /// All policies run before returning, and all registered mutations are
    /// rolled back in reverse order.
    pub fn execute(self) -> Result<Reservation, Vec<Reject>> {
        self.inner.execute()
    }

    pub(crate) fn from_handle(inner: Box<dyn RequestHandle>) -> Self {
        Self { inner }
    }
}

#[cfg(test)]
mod tests {
    use super::Request;
    use crate::pretrade::handles::RequestHandleImpl;
    use crate::pretrade::Reservation;

    #[test]
    fn execute_consumes_request_and_delegates_to_handle() {
        let request = Request::from_handle(Box::new(RequestHandleImpl::new(Box::new(|| {
            Ok(Reservation::from_handle(Box::new(NoopReservationHandle)))
        }))));

        let reservation = request.execute().expect("request execution must succeed");
        reservation.commit();
    }

    #[test]
    fn execute_can_finalize_returned_reservation_with_rollback() {
        let request = Request::from_handle(Box::new(RequestHandleImpl::new(Box::new(|| {
            Ok(Reservation::from_handle(Box::new(NoopReservationHandle)))
        }))));

        let reservation = request.execute().expect("request execution must succeed");
        reservation.rollback();
    }

    struct NoopReservationHandle;

    impl crate::pretrade::reservation::ReservationHandle for NoopReservationHandle {
        fn commit(self: Box<Self>) {}

        fn rollback(self: Box<Self>) {}
    }
}
