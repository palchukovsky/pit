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

//! Runtime wrapper for the account-adjustment amount group.

use openpit::param::AdjustmentAmount;
use openpit::{
    AccountAdjustmentAmount, HasAccountAdjustmentBalance, HasAccountAdjustmentHeld,
    HasAccountAdjustmentIncoming, RequestFieldAccessError,
};

/// Runtime access to an account adjustment's amount group.
///
/// Use [`AccountAdjustmentAmountAccess::Populated`] when the group is present,
/// [`AccountAdjustmentAmountAccess::Absent`] when it is not.
///
/// When absent, all three traits return `Err`; within a populated group, each
/// individual amount is `Option<AdjustmentAmount>` and may be `None`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AccountAdjustmentAmountAccess {
    /// The amount group is present.
    Populated(AccountAdjustmentAmount),
    /// The amount group is absent.
    Absent,
}

impl HasAccountAdjustmentBalance for AccountAdjustmentAmountAccess {
    fn balance(&self) -> Result<Option<AdjustmentAmount>, RequestFieldAccessError> {
        match self {
            Self::Populated(a) => Ok(a.balance),
            Self::Absent => Err(RequestFieldAccessError::new("amount.balance")),
        }
    }
}

impl HasAccountAdjustmentHeld for AccountAdjustmentAmountAccess {
    fn held(&self) -> Result<Option<AdjustmentAmount>, RequestFieldAccessError> {
        match self {
            Self::Populated(a) => Ok(a.held),
            Self::Absent => Err(RequestFieldAccessError::new("amount.held")),
        }
    }
}

impl HasAccountAdjustmentIncoming for AccountAdjustmentAmountAccess {
    fn incoming(&self) -> Result<Option<AdjustmentAmount>, RequestFieldAccessError> {
        match self {
            Self::Populated(a) => Ok(a.incoming),
            Self::Absent => Err(RequestFieldAccessError::new("amount.incoming")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpit::param::{AdjustmentAmount, PositionSize};
    use openpit::AccountAdjustmentAmount;

    #[test]
    fn populated_returns_ok_with_some_values() {
        let access = AccountAdjustmentAmountAccess::Populated(AccountAdjustmentAmount {
            balance: Some(AdjustmentAmount::Absolute(
                PositionSize::from_str("10").expect("valid"),
            )),
            held: None,
            incoming: None,
        });
        assert!(access.balance().unwrap().is_some());
        assert!(access.held().unwrap().is_none());
        assert!(access.incoming().unwrap().is_none());
    }

    #[test]
    fn absent_returns_err() {
        let access = AccountAdjustmentAmountAccess::Absent;
        assert!(access.balance().is_err());
        assert!(access.held().is_err());
        assert!(access.incoming().is_err());
    }
}
