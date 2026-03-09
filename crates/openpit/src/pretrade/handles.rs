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

use super::mutations::{Mutation, RiskMutation};
use super::reject::Reject;
use super::request::RequestHandle;
use super::reservation::{Reservation, ReservationHandle};

type RequestExecutor = Box<dyn FnOnce() -> Result<Reservation, Vec<Reject>>>;
type MutationApplier = Box<dyn FnMut(&RiskMutation)>;

pub(crate) struct RequestHandleImpl {
    execute: RequestExecutor,
}

impl RequestHandleImpl {
    pub(crate) fn new(execute: RequestExecutor) -> Self {
        Self { execute }
    }
}

impl RequestHandle for RequestHandleImpl {
    fn execute(self: Box<Self>) -> Result<Reservation, Vec<Reject>> {
        let this = *self;
        (this.execute)()
    }
}

pub(crate) struct ReservationHandleImpl {
    mutations: Vec<Mutation>,
    apply_mutation: MutationApplier,
}

impl ReservationHandleImpl {
    pub(crate) fn new(mutations: Vec<Mutation>, apply_mutation: MutationApplier) -> Self {
        Self {
            mutations,
            apply_mutation,
        }
    }
}

impl ReservationHandle for ReservationHandleImpl {
    fn commit(mut self: Box<Self>) {
        apply_commit_mutations(&self.mutations, &mut self.apply_mutation);
    }

    fn rollback(mut self: Box<Self>) {
        apply_rollback_mutations(&self.mutations, &mut self.apply_mutation);
    }
}

fn apply_commit_mutations(mutations: &[Mutation], apply_mutation: &mut dyn FnMut(&RiskMutation)) {
    for mutation in mutations {
        apply_mutation(&mutation.commit);
    }
}

fn apply_rollback_mutations(mutations: &[Mutation], apply_mutation: &mut dyn FnMut(&RiskMutation)) {
    for mutation in mutations.iter().rev() {
        apply_mutation(&mutation.rollback);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::ReservationHandleImpl;
    use crate::param::{Asset, Volume};
    use crate::pretrade::reservation::ReservationHandle;
    use crate::pretrade::{Mutation, RiskMutation};

    fn mutation_pair(id: &'static str) -> Mutation {
        Mutation {
            commit: RiskMutation::SetKillSwitch { id, enabled: true },
            rollback: RiskMutation::SetKillSwitch { id, enabled: false },
        }
    }

    fn reserve_notional_pair() -> Mutation {
        Mutation {
            commit: RiskMutation::ReserveNotional {
                asset: Asset::new("USD").expect("asset code must be valid"),
                amount: Volume::from_str("10").expect("volume must be valid"),
            },
            rollback: RiskMutation::ReserveNotional {
                asset: Asset::new("USD").expect("asset code must be valid"),
                amount: Volume::from_str("10").expect("volume must be valid"),
            },
        }
    }

    #[test]
    fn commit_applies_mutations_in_registration_order() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let calls_clone = Rc::clone(&calls);
        let apply = Box::new(move |mutation: &RiskMutation| {
            if let RiskMutation::SetKillSwitch { id, enabled } = mutation {
                calls_clone.borrow_mut().push((*id, *enabled));
            }
        });

        let handle = Box::new(ReservationHandleImpl::new(
            vec![
                reserve_notional_pair(),
                mutation_pair("m1"),
                mutation_pair("m2"),
            ],
            apply,
        ));
        handle.commit();

        assert_eq!(&*calls.borrow(), &[("m1", true), ("m2", true)]);
    }

    #[test]
    fn rollback_applies_mutations_in_reverse_registration_order() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let calls_clone = Rc::clone(&calls);
        let apply = Box::new(move |mutation: &RiskMutation| {
            if let RiskMutation::SetKillSwitch { id, enabled } = mutation {
                calls_clone.borrow_mut().push((*id, *enabled));
            }
        });

        let handle = Box::new(ReservationHandleImpl::new(
            vec![
                mutation_pair("m1"),
                mutation_pair("m2"),
                reserve_notional_pair(),
            ],
            apply,
        ));
        handle.rollback();

        assert_eq!(&*calls.borrow(), &[("m2", false), ("m1", false)]);
    }

    #[test]
    fn commit_observer_can_ignore_non_kill_switch_mutations() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let calls_clone = Rc::clone(&calls);
        let apply = Box::new(move |mutation: &RiskMutation| {
            if let RiskMutation::SetKillSwitch { id, enabled } = mutation {
                calls_clone.borrow_mut().push((*id, *enabled));
            }
        });

        let handle = Box::new(ReservationHandleImpl::new(
            vec![reserve_notional_pair(), mutation_pair("m1")],
            apply,
        ));
        handle.commit();

        assert_eq!(&*calls.borrow(), &[("m1", true)]);
    }

    #[test]
    fn rollback_observer_can_ignore_non_kill_switch_mutations() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let calls_clone = Rc::clone(&calls);
        let apply = Box::new(move |mutation: &RiskMutation| {
            if let RiskMutation::SetKillSwitch { id, enabled } = mutation {
                calls_clone.borrow_mut().push((*id, *enabled));
            }
        });

        let handle = Box::new(ReservationHandleImpl::new(
            vec![mutation_pair("m1"), reserve_notional_pair()],
            apply,
        ));
        handle.rollback();

        assert_eq!(&*calls.borrow(), &[("m1", false)]);
    }
}
