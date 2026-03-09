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

/// Opaque capability object representing reserved state.
///
/// If dropped without explicit finalization, rollback is executed automatically.
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
/// let reservation = engine
///     .start_pre_trade(order)
///     .expect("start stage must pass")
///     .execute()
///     .expect("main stage must pass");
///
/// // Send order to venue. On success commit, on failure rollback.
/// reservation.commit(); // or reservation.rollback()
/// ```
pub struct Reservation {
    inner: Option<Box<dyn ReservationHandle>>,
}

/// Internal capability interface used by [`Reservation`].
pub(crate) trait ReservationHandle {
    /// Finalizes the reservation by applying commit mutations.
    fn commit(self: Box<Self>);
    /// Finalizes the reservation by applying rollback mutations.
    fn rollback(self: Box<Self>);
}

impl Reservation {
    /// Finalizes by applying commit mutations.
    pub fn commit(mut self) {
        if let Some(inner) = self.inner.take() {
            inner.commit();
        }
    }

    /// Finalizes by applying rollback mutations.
    pub fn rollback(mut self) {
        if let Some(inner) = self.inner.take() {
            inner.rollback();
        }
    }

    pub(crate) fn from_handle(inner: Box<dyn ReservationHandle>) -> Self {
        Self { inner: Some(inner) }
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            inner.rollback();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::Reservation;
    use crate::param::{Asset, Volume};
    use crate::pretrade::handles::ReservationHandleImpl;
    use crate::pretrade::{Mutation, RiskMutation};

    #[test]
    fn drop_without_explicit_finalize_rolls_back() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let calls_clone = Rc::clone(&calls);
        let apply = Box::new(move |mutation: &RiskMutation| {
            if let RiskMutation::SetKillSwitch { id, enabled } = mutation {
                calls_clone.borrow_mut().push((*id, *enabled));
            }
        });

        let reservation = Reservation::from_handle(Box::new(ReservationHandleImpl::new(
            vec![
                Mutation {
                    commit: RiskMutation::SetKillSwitch {
                        id: "m1",
                        enabled: true,
                    },
                    rollback: RiskMutation::SetKillSwitch {
                        id: "m1",
                        enabled: false,
                    },
                },
                Mutation {
                    commit: RiskMutation::ReserveNotional {
                        asset: Asset::new("USD").expect("asset code must be valid"),
                        amount: Volume::from_str("10").expect("volume must be valid"),
                    },
                    rollback: RiskMutation::ReserveNotional {
                        asset: Asset::new("USD").expect("asset code must be valid"),
                        amount: Volume::from_str("10").expect("volume must be valid"),
                    },
                },
            ],
            apply,
        )));

        drop(reservation);

        assert_eq!(&*calls.borrow(), &[("m1", false)]);
    }

    #[test]
    fn drop_without_explicit_finalize_can_ignore_non_kill_switch_mutations() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let calls_clone = Rc::clone(&calls);
        let apply = Box::new(move |mutation: &RiskMutation| {
            if let RiskMutation::SetKillSwitch { id, enabled } = mutation {
                calls_clone.borrow_mut().push((*id, *enabled));
            }
        });

        let reservation = Reservation::from_handle(Box::new(ReservationHandleImpl::new(
            vec![
                Mutation {
                    commit: RiskMutation::SetKillSwitch {
                        id: "m1",
                        enabled: true,
                    },
                    rollback: RiskMutation::SetKillSwitch {
                        id: "m1",
                        enabled: false,
                    },
                },
                Mutation {
                    commit: RiskMutation::ReserveNotional {
                        asset: Asset::new("USD").expect("asset code must be valid"),
                        amount: Volume::from_str("10").expect("volume must be valid"),
                    },
                    rollback: RiskMutation::ReserveNotional {
                        asset: Asset::new("USD").expect("asset code must be valid"),
                        amount: Volume::from_str("10").expect("volume must be valid"),
                    },
                },
            ],
            apply,
        )));

        drop(reservation);

        assert_eq!(&*calls.borrow(), &[("m1", false)]);
    }

    #[test]
    fn commit_is_noop_for_finalized_reservation() {
        let reservation = Reservation { inner: None };
        reservation.commit();
    }

    #[test]
    fn rollback_is_noop_for_finalized_reservation() {
        let reservation = Reservation { inner: None };
        reservation.rollback();
    }
}
