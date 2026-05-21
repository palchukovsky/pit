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

//! Runtime wrapper for the account-adjustment bounds group.

use openpit::param::PositionSize;
use openpit::{
    AccountAdjustmentBounds, HasAccountAdjustmentBalanceLowerBound,
    HasAccountAdjustmentBalanceUpperBound, HasAccountAdjustmentHeldLowerBound,
    HasAccountAdjustmentHeldUpperBound, HasAccountAdjustmentIncomingLowerBound,
    HasAccountAdjustmentIncomingUpperBound, RequestFieldAccessError,
};

/// Runtime access to an account adjustment's bounds group.
///
/// Use [`AccountAdjustmentBoundsAccess::Populated`] when the group is present,
/// [`AccountAdjustmentBoundsAccess::Absent`] when it is not.
///
/// When absent, all six traits return `Err`; within a populated group, each
/// individual bound is `Option<PositionSize>` and may be `None`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AccountAdjustmentBoundsAccess {
    /// The bounds group is present.
    Populated(AccountAdjustmentBounds),
    /// The bounds group is absent.
    Absent,
}

impl HasAccountAdjustmentBalanceUpperBound for AccountAdjustmentBoundsAccess {
    fn balance_upper(&self) -> Result<Option<PositionSize>, RequestFieldAccessError> {
        match self {
            Self::Populated(b) => Ok(b.balance_upper),
            Self::Absent => Err(RequestFieldAccessError::new("bounds.balance_upper")),
        }
    }
}

impl HasAccountAdjustmentBalanceLowerBound for AccountAdjustmentBoundsAccess {
    fn balance_lower(&self) -> Result<Option<PositionSize>, RequestFieldAccessError> {
        match self {
            Self::Populated(b) => Ok(b.balance_lower),
            Self::Absent => Err(RequestFieldAccessError::new("bounds.balance_lower")),
        }
    }
}

impl HasAccountAdjustmentHeldUpperBound for AccountAdjustmentBoundsAccess {
    fn held_upper(&self) -> Result<Option<PositionSize>, RequestFieldAccessError> {
        match self {
            Self::Populated(b) => Ok(b.held_upper),
            Self::Absent => Err(RequestFieldAccessError::new("bounds.held_upper")),
        }
    }
}

impl HasAccountAdjustmentHeldLowerBound for AccountAdjustmentBoundsAccess {
    fn held_lower(&self) -> Result<Option<PositionSize>, RequestFieldAccessError> {
        match self {
            Self::Populated(b) => Ok(b.held_lower),
            Self::Absent => Err(RequestFieldAccessError::new("bounds.held_lower")),
        }
    }
}

impl HasAccountAdjustmentIncomingUpperBound for AccountAdjustmentBoundsAccess {
    fn incoming_upper(&self) -> Result<Option<PositionSize>, RequestFieldAccessError> {
        match self {
            Self::Populated(b) => Ok(b.incoming_upper),
            Self::Absent => Err(RequestFieldAccessError::new("bounds.incoming_upper")),
        }
    }
}

impl HasAccountAdjustmentIncomingLowerBound for AccountAdjustmentBoundsAccess {
    fn incoming_lower(&self) -> Result<Option<PositionSize>, RequestFieldAccessError> {
        match self {
            Self::Populated(b) => Ok(b.incoming_lower),
            Self::Absent => Err(RequestFieldAccessError::new("bounds.incoming_lower")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpit::param::PositionSize;
    use openpit::AccountAdjustmentBounds;

    #[test]
    fn populated_returns_ok() {
        let access = AccountAdjustmentBoundsAccess::Populated(AccountAdjustmentBounds {
            balance_upper: Some(PositionSize::from_str("100").expect("valid")),
            balance_lower: Some(PositionSize::from_str("-10").expect("valid")),
            held_upper: Some(PositionSize::from_str("90").expect("valid")),
            held_lower: Some(PositionSize::from_str("-9").expect("valid")),
            incoming_upper: Some(PositionSize::from_str("80").expect("valid")),
            incoming_lower: Some(PositionSize::from_str("-8").expect("valid")),
        });
        assert!(access.balance_upper().unwrap().is_some());
        assert!(access.balance_lower().unwrap().is_some());
        assert!(access.held_upper().unwrap().is_some());
        assert!(access.held_lower().unwrap().is_some());
        assert!(access.incoming_upper().unwrap().is_some());
        assert!(access.incoming_lower().unwrap().is_some());
    }

    #[test]
    fn absent_returns_err_for_all() {
        let access = AccountAdjustmentBoundsAccess::Absent;
        assert!(access.balance_upper().is_err());
        assert!(access.balance_lower().is_err());
        assert!(access.held_upper().is_err());
        assert!(access.held_lower().is_err());
        assert!(access.incoming_upper().is_err());
        assert!(access.incoming_lower().is_err());
    }
}
