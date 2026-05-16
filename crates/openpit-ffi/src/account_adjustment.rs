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

#![allow(clippy::missing_safety_doc, clippy::not_unsafe_ptr_arg_deref)]

use std::ffi::c_void;

use openpit::param::{AdjustmentAmount, PositionSize};
use openpit::{
    AccountAdjustmentAmount, AccountAdjustmentBalanceOperation, AccountAdjustmentBounds,
    AccountAdjustmentPositionOperation,
};
use openpit_interop::{
    AccountAdjustmentAmountAccess, AccountAdjustmentBoundsAccess, AccountAdjustmentOperationAccess,
    PopulatedAccountAdjustmentOperation, RequestWithPayload,
};

use crate::define_optional;
use crate::instrument::{import_instrument, parse_asset_view, OpenPitInstrument};
use crate::param::{
    export_leverage, export_position_mode, import_leverage, import_position_mode,
    OpenPitParamAdjustmentAmountKind, OpenPitParamLeverage, OpenPitParamPositionMode,
    OpenPitParamPositionSize, OpenPitParamPositionSizeOptional, OpenPitParamPrice,
    OpenPitParamPriceOptional,
};
use crate::OpenPitStringView;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
/// One amount component inside an account adjustment.
///
/// The numeric value is interpreted according to `kind`:
/// - `Delta` means "change current state by this signed amount";
/// - `Absolute` means "set current state to this signed amount".
pub struct OpenPitParamAdjustmentAmount {
    /// Signed numeric value of the adjustment.
    pub value: OpenPitParamPositionSize,
    /// Interpretation mode for `value`.
    pub kind: OpenPitParamAdjustmentAmountKind,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
/// Balance-operation payload for account adjustment.
pub struct OpenPitAccountAdjustmentBalanceOperation {
    /// Balance asset code.
    pub asset: OpenPitStringView,
    /// Optional average entry price.
    pub average_entry_price: OpenPitParamPriceOptional,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
/// Position-operation payload for account adjustment.
pub struct OpenPitAccountAdjustmentPositionOperation {
    /// Position instrument.
    pub instrument: OpenPitInstrument,
    /// Position collateral asset.
    pub collateral_asset: OpenPitStringView,
    /// Position average entry price.
    pub average_entry_price: OpenPitParamPriceOptional,
    /// Optional leverage.
    pub leverage: OpenPitParamLeverage,
    /// Position mode.
    pub mode: OpenPitParamPositionMode,
}

define_optional!(
    optional = OpenPitAccountAdjustmentBalanceOperationOptional,
    value = OpenPitAccountAdjustmentBalanceOperation
);
define_optional!(
    optional = OpenPitAccountAdjustmentPositionOperationOptional,
    value = OpenPitAccountAdjustmentPositionOperation
);

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
/// Optional amount-change group for account adjustment.
///
/// The group is absent when every field is absent.
pub struct OpenPitAccountAdjustmentAmount {
    /// Requested total-balance change.
    pub total: OpenPitParamAdjustmentAmount,
    /// Requested reserved-balance change.
    pub reserved: OpenPitParamAdjustmentAmount,
    /// Requested pending-balance change.
    pub pending: OpenPitParamAdjustmentAmount,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
/// Optional bounds group for account adjustment.
///
/// The group is absent when every bound is absent.
pub struct OpenPitAccountAdjustmentBounds {
    /// Optional upper bound for total balance.
    pub total_upper: OpenPitParamPositionSizeOptional,
    /// Optional lower bound for total balance.
    pub total_lower: OpenPitParamPositionSizeOptional,
    /// Optional upper bound for reserved balance.
    pub reserved_upper: OpenPitParamPositionSizeOptional,
    /// Optional lower bound for reserved balance.
    pub reserved_lower: OpenPitParamPositionSizeOptional,
    /// Optional upper bound for pending balance.
    pub pending_upper: OpenPitParamPositionSizeOptional,
    /// Optional lower bound for pending balance.
    pub pending_lower: OpenPitParamPositionSizeOptional,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
/// Full caller-owned account-adjustment payload.
pub struct OpenPitAccountAdjustment {
    /// Optional balance-operation group.
    pub balance_operation: OpenPitAccountAdjustmentBalanceOperationOptional,
    /// Optional position-operation group.
    pub position_operation: OpenPitAccountAdjustmentPositionOperationOptional,
    /// Optional amount-change group.
    pub amount: OpenPitAccountAdjustmentAmountOptional,
    /// Optional bounds group.
    pub bounds: OpenPitAccountAdjustmentBoundsOptional,
    /// Opaque caller-defined token.
    ///
    /// The SDK never inspects, dereferences, or frees this value. Its meaning,
    /// lifetime, and thread-safety are the caller's responsibility. `0` / null
    /// means "not set". See the project Threading Contract for the full lifetime
    /// model.
    ///
    /// The token is preserved unchanged across every engine callback that
    /// receives the carrying value, including policy callbacks and adjustment
    /// callbacks.
    pub user_data: *mut c_void,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
/// Result of `openpit_engine_apply_account_adjustment`.
pub enum OpenPitAccountAdjustmentApplyStatus {
    /// The call failed before the batch could be evaluated.
    #[default]
    Error = 0,
    /// The batch was accepted and applied.
    Applied = 1,
    /// The batch was evaluated and rejected by policy or validation logic.
    Rejected = 2,
}

define_optional!(
    optional = OpenPitAccountAdjustmentAmountOptional,
    value = OpenPitAccountAdjustmentAmount
);
define_optional!(
    optional = OpenPitAccountAdjustmentBoundsOptional,
    value = OpenPitAccountAdjustmentBounds
);

fn import_adjustment_amount(
    value: OpenPitParamAdjustmentAmount,
) -> Result<Option<AdjustmentAmount>, String> {
    match value.kind {
        OpenPitParamAdjustmentAmountKind::NotSet => Ok(None),
        OpenPitParamAdjustmentAmountKind::Delta => {
            Ok(Some(AdjustmentAmount::Delta(value.value.to_param()?)))
        }
        OpenPitParamAdjustmentAmountKind::Absolute => {
            Ok(Some(AdjustmentAmount::Absolute(value.value.to_param()?)))
        }
    }
}

fn export_adjustment_amount(value: Option<AdjustmentAmount>) -> OpenPitParamAdjustmentAmount {
    match value {
        Some(AdjustmentAmount::Delta(v)) => OpenPitParamAdjustmentAmount {
            kind: OpenPitParamAdjustmentAmountKind::Delta,
            value: OpenPitParamPositionSize(v.to_decimal().into()),
        },
        Some(AdjustmentAmount::Absolute(v)) => OpenPitParamAdjustmentAmount {
            kind: OpenPitParamAdjustmentAmountKind::Absolute,
            value: OpenPitParamPositionSize(v.to_decimal().into()),
        },
        _ => OpenPitParamAdjustmentAmount::default(),
    }
}

fn import_balance_operation(
    value: OpenPitAccountAdjustmentBalanceOperationOptional,
) -> Result<Option<AccountAdjustmentBalanceOperation>, String> {
    if !value.is_set {
        return Ok(None);
    }

    let asset = parse_asset_view(value.value.asset, "account_adjustment.balance.asset")?
        .ok_or_else(|| "account_adjustment.balance.asset is not set".to_string())?;

    let average_entry_price = if value.value.average_entry_price.is_set {
        Some(value.value.average_entry_price.value.to_param()?)
    } else {
        None
    };

    Ok(Some(AccountAdjustmentBalanceOperation {
        asset,
        average_entry_price,
    }))
}

fn import_position_operation(
    value: OpenPitAccountAdjustmentPositionOperationOptional,
) -> Result<Option<AccountAdjustmentPositionOperation>, String> {
    if !value.is_set {
        return Ok(None);
    }

    let instrument = import_instrument(&value.value.instrument)?
        .ok_or_else(|| "account_adjustment.position.instrument is not set".to_string())?;
    let collateral_asset = parse_asset_view(
        value.value.collateral_asset,
        "account_adjustment.position.collateral_asset",
    )?
    .ok_or_else(|| "account_adjustment.position.collateral_asset is not set".to_string())?;
    let average_entry_price = if value.value.average_entry_price.is_set {
        value.value.average_entry_price.value.to_param()?
    } else {
        return Err("account_adjustment.position.average_entry_price is not set".to_string());
    };
    let mode = import_position_mode(value.value.mode)
        .ok_or_else(|| "account_adjustment.position.mode is not set".to_string())?;

    Ok(Some(AccountAdjustmentPositionOperation {
        instrument,
        collateral_asset,
        average_entry_price,
        mode,
        leverage: import_leverage(value.value.leverage),
    }))
}

fn import_amount(
    value: OpenPitAccountAdjustmentAmountOptional,
) -> Result<AccountAdjustmentAmountAccess, String> {
    if !value.is_set {
        return Ok(AccountAdjustmentAmountAccess::Absent);
    }

    Ok(AccountAdjustmentAmountAccess::Populated(
        AccountAdjustmentAmount {
            total: import_adjustment_amount(value.value.total)?,
            reserved: import_adjustment_amount(value.value.reserved)?,
            pending: import_adjustment_amount(value.value.pending)?,
        },
    ))
}

fn import_bound(value: OpenPitParamPositionSizeOptional) -> Result<Option<PositionSize>, String> {
    if !value.is_set {
        return Ok(None);
    }
    Ok(Some(value.value.to_param()?))
}

fn import_bounds(
    value: OpenPitAccountAdjustmentBoundsOptional,
) -> Result<AccountAdjustmentBoundsAccess, String> {
    if !value.is_set {
        return Ok(AccountAdjustmentBoundsAccess::Absent);
    }

    Ok(AccountAdjustmentBoundsAccess::Populated(
        AccountAdjustmentBounds {
            total_upper: import_bound(value.value.total_upper)?,
            total_lower: import_bound(value.value.total_lower)?,
            reserved_upper: import_bound(value.value.reserved_upper)?,
            reserved_lower: import_bound(value.value.reserved_lower)?,
            pending_upper: import_bound(value.value.pending_upper)?,
            pending_lower: import_bound(value.value.pending_lower)?,
        },
    ))
}

fn export_balance_operation(
    value: &AccountAdjustmentBalanceOperation,
) -> OpenPitAccountAdjustmentBalanceOperation {
    OpenPitAccountAdjustmentBalanceOperation {
        asset: OpenPitStringView::from_utf8(value.asset.as_ref()),
        average_entry_price: match value.average_entry_price {
            Some(v) => OpenPitParamPriceOptional {
                is_set: true,
                value: OpenPitParamPrice(v.to_decimal().into()),
            },
            None => OpenPitParamPriceOptional::default(),
        },
    }
}

fn export_position_operation(
    value: &AccountAdjustmentPositionOperation,
) -> OpenPitAccountAdjustmentPositionOperation {
    OpenPitAccountAdjustmentPositionOperation {
        instrument: OpenPitInstrument {
            underlying_asset: OpenPitStringView::from_utf8(
                value.instrument.underlying_asset().as_ref(),
            ),
            settlement_asset: OpenPitStringView::from_utf8(
                value.instrument.settlement_asset().as_ref(),
            ),
        },
        collateral_asset: OpenPitStringView::from_utf8(value.collateral_asset.as_ref()),
        average_entry_price: OpenPitParamPriceOptional {
            is_set: true,
            value: OpenPitParamPrice(value.average_entry_price.to_decimal().into()),
        },
        leverage: export_leverage(value.leverage),
        mode: export_position_mode(value.mode),
    }
}

fn export_amount(value: &AccountAdjustmentAmount) -> OpenPitAccountAdjustmentAmount {
    OpenPitAccountAdjustmentAmount {
        total: export_adjustment_amount(value.total),
        reserved: export_adjustment_amount(value.reserved),
        pending: export_adjustment_amount(value.pending),
    }
}

fn export_bound(value: Option<PositionSize>) -> OpenPitParamPositionSizeOptional {
    match value {
        Some(v) => OpenPitParamPositionSizeOptional {
            is_set: true,
            value: OpenPitParamPositionSize(v.to_decimal().into()),
        },
        None => OpenPitParamPositionSizeOptional::default(),
    }
}

fn export_bounds(value: &AccountAdjustmentBounds) -> OpenPitAccountAdjustmentBounds {
    OpenPitAccountAdjustmentBounds {
        total_upper: export_bound(value.total_upper),
        total_lower: export_bound(value.total_lower),
        reserved_upper: export_bound(value.reserved_upper),
        reserved_lower: export_bound(value.reserved_lower),
        pending_upper: export_bound(value.pending_upper),
        pending_lower: export_bound(value.pending_lower),
    }
}

pub(crate) fn import_account_adjustment(
    value: &OpenPitAccountAdjustment,
) -> Result<AccountAdjustment, String> {
    // The engine applies adjustments as owned domain values, so decoding a
    // borrowed adjustment view necessarily builds owned data here.
    let balance_operation = import_balance_operation(value.balance_operation)?;
    let position_operation = import_position_operation(value.position_operation)?;

    let operation = match (balance_operation, position_operation) {
        (Some(balance), None) => AccountAdjustmentOperationAccess::Populated(
            PopulatedAccountAdjustmentOperation::Balance(balance),
        ),
        (None, Some(position)) => AccountAdjustmentOperationAccess::Populated(
            PopulatedAccountAdjustmentOperation::Position(position),
        ),
        (None, None) => AccountAdjustmentOperationAccess::Absent,
        (Some(_), Some(_)) => {
            return Err("account_adjustment has both balance and position operation".to_string())
        }
    };

    Ok(RequestWithPayload::new(
        openpit_interop::AccountAdjustment {
            operation,
            amount: import_amount(value.amount)?,
            bounds: import_bounds(value.bounds)?,
        },
        value.user_data,
    ))
}

pub(crate) fn export_account_adjustment(value: &AccountAdjustment) -> OpenPitAccountAdjustment {
    let (balance_operation, position_operation) = match &value.request.operation {
        AccountAdjustmentOperationAccess::Populated(
            PopulatedAccountAdjustmentOperation::Balance(v),
        ) => (
            OpenPitAccountAdjustmentBalanceOperationOptional {
                value: export_balance_operation(v),
                is_set: true,
            },
            OpenPitAccountAdjustmentPositionOperationOptional::default(),
        ),
        AccountAdjustmentOperationAccess::Populated(
            PopulatedAccountAdjustmentOperation::Position(v),
        ) => (
            OpenPitAccountAdjustmentBalanceOperationOptional::default(),
            OpenPitAccountAdjustmentPositionOperationOptional {
                value: export_position_operation(v),
                is_set: true,
            },
        ),
        AccountAdjustmentOperationAccess::Absent => (
            OpenPitAccountAdjustmentBalanceOperationOptional::default(),
            OpenPitAccountAdjustmentPositionOperationOptional::default(),
        ),
    };

    OpenPitAccountAdjustment {
        balance_operation,
        position_operation,
        amount: match &value.request.amount {
            AccountAdjustmentAmountAccess::Populated(v) => OpenPitAccountAdjustmentAmountOptional {
                value: export_amount(v),
                is_set: true,
            },
            AccountAdjustmentAmountAccess::Absent => {
                OpenPitAccountAdjustmentAmountOptional::default()
            }
        },
        bounds: match &value.request.bounds {
            AccountAdjustmentBoundsAccess::Populated(v) => OpenPitAccountAdjustmentBoundsOptional {
                value: export_bounds(v),
                is_set: true,
            },
            AccountAdjustmentBoundsAccess::Absent => {
                OpenPitAccountAdjustmentBoundsOptional::default()
            }
        },
        user_data: value.payload,
    }
}

/// FFI account-adjustment request paired with an opaque caller-defined token.
///
/// The token is stored in [`RequestWithPayload::payload`]. The SDK never
/// inspects, dereferences, or frees this value. Its meaning, lifetime, and
/// thread-safety are the caller's responsibility. A null pointer means
/// "not set". See the project Threading Contract for the full lifetime model.
///
/// The token is preserved unchanged across every engine callback that
/// receives the carrying value, including policy callbacks and adjustment
/// callbacks.
pub type AccountAdjustment = RequestWithPayload<openpit_interop::AccountAdjustment, *mut c_void>;

#[cfg(test)]
mod tests {
    use crate::OpenPitStringView;

    use super::{
        export_account_adjustment, import_account_adjustment, OpenPitAccountAdjustment,
        OpenPitAccountAdjustmentAmount, OpenPitAccountAdjustmentAmountOptional,
        OpenPitAccountAdjustmentBalanceOperation, OpenPitAccountAdjustmentBalanceOperationOptional,
        OpenPitAccountAdjustmentBounds, OpenPitAccountAdjustmentBoundsOptional,
        OpenPitAccountAdjustmentPositionOperation,
        OpenPitAccountAdjustmentPositionOperationOptional, OpenPitParamAdjustmentAmount,
    };
    use crate::instrument::OpenPitInstrument;
    use crate::param::{
        OpenPitParamAdjustmentAmountKind, OpenPitParamPositionMode, OpenPitParamPositionSize,
        OpenPitParamPositionSizeOptional, OpenPitParamPrice,
    };
    use openpit::param::{AdjustmentAmount, Asset, PositionMode, PositionSize, Price};
    use openpit::{
        AccountAdjustmentAmount, AccountAdjustmentBalanceOperation, AccountAdjustmentBounds,
        AccountAdjustmentPositionOperation, Instrument,
    };
    use openpit_interop::{
        AccountAdjustmentAmountAccess, AccountAdjustmentBoundsAccess,
        AccountAdjustmentOperationAccess, PopulatedAccountAdjustmentOperation, RequestWithPayload,
    };

    fn sample_balance_adjustment() -> OpenPitAccountAdjustment {
        OpenPitAccountAdjustment {
            balance_operation: OpenPitAccountAdjustmentBalanceOperationOptional {
                value: OpenPitAccountAdjustmentBalanceOperation {
                    asset: OpenPitStringView {
                        ptr: b"USD".as_ptr(),
                        len: 3,
                    },
                    average_entry_price: crate::param::OpenPitParamPriceOptional {
                        value: OpenPitParamPrice(
                            Price::from_str("10").expect("price").to_decimal().into(),
                        ),
                        is_set: true,
                    },
                },
                is_set: true,
            },
            position_operation: OpenPitAccountAdjustmentPositionOperationOptional::default(),
            amount: OpenPitAccountAdjustmentAmountOptional {
                is_set: true,
                value: OpenPitAccountAdjustmentAmount {
                    total: OpenPitParamAdjustmentAmount {
                        value: OpenPitParamPositionSize(
                            PositionSize::from_str("1")
                                .expect("size")
                                .to_decimal()
                                .into(),
                        ),
                        kind: OpenPitParamAdjustmentAmountKind::Delta,
                    },
                    reserved: OpenPitParamAdjustmentAmount {
                        value: OpenPitParamPositionSize(
                            PositionSize::from_str("2")
                                .expect("size")
                                .to_decimal()
                                .into(),
                        ),
                        kind: OpenPitParamAdjustmentAmountKind::Absolute,
                    },
                    pending: OpenPitParamAdjustmentAmount::default(),
                },
            },
            bounds: OpenPitAccountAdjustmentBoundsOptional {
                is_set: true,
                value: OpenPitAccountAdjustmentBounds {
                    total_upper: OpenPitParamPositionSizeOptional {
                        is_set: true,
                        value: OpenPitParamPositionSize(
                            PositionSize::from_str("100")
                                .expect("size")
                                .to_decimal()
                                .into(),
                        ),
                    },
                    total_lower: OpenPitParamPositionSizeOptional::default(),
                    reserved_upper: OpenPitParamPositionSizeOptional::default(),
                    reserved_lower: OpenPitParamPositionSizeOptional::default(),
                    pending_upper: OpenPitParamPositionSizeOptional::default(),
                    pending_lower: OpenPitParamPositionSizeOptional::default(),
                },
            },
            user_data: std::ptr::null_mut(),
        }
    }

    #[test]
    fn import_account_adjustment_accepts_balance_payload() {
        let imported = import_account_adjustment(&sample_balance_adjustment()).expect("import");

        let operation =
            if let AccountAdjustmentOperationAccess::Populated(op) = &imported.request.operation {
                op
            } else {
                panic!("operation must be populated");
            };
        assert_eq!(
            *operation,
            PopulatedAccountAdjustmentOperation::Balance(AccountAdjustmentBalanceOperation {
                asset: Asset::new("USD").expect("asset"),
                average_entry_price: Some(Price::from_str("10").expect("price")),
            })
        );

        let amount = if let AccountAdjustmentAmountAccess::Populated(a) = &imported.request.amount {
            a
        } else {
            panic!("amount must be populated");
        };
        assert_eq!(
            *amount,
            AccountAdjustmentAmount {
                total: Some(AdjustmentAmount::Delta(
                    PositionSize::from_str("1").expect("size"),
                )),
                reserved: Some(AdjustmentAmount::Absolute(
                    PositionSize::from_str("2").expect("size"),
                )),
                pending: None,
            }
        );

        let bounds = if let AccountAdjustmentBoundsAccess::Populated(b) = &imported.request.bounds {
            b
        } else {
            panic!("bounds must be populated");
        };
        assert_eq!(
            *bounds,
            AccountAdjustmentBounds {
                total_upper: Some(PositionSize::from_str("100").expect("size")),
                total_lower: None,
                reserved_upper: None,
                reserved_lower: None,
                pending_upper: None,
                pending_lower: None,
            }
        );
    }

    #[test]
    fn import_account_adjustment_rejects_both_operations() {
        let mut input = sample_balance_adjustment();
        input.position_operation = OpenPitAccountAdjustmentPositionOperationOptional {
            value: OpenPitAccountAdjustmentPositionOperation {
                instrument: OpenPitInstrument {
                    underlying_asset: OpenPitStringView {
                        ptr: b"AAPL".as_ptr(),
                        len: 4,
                    },
                    settlement_asset: OpenPitStringView {
                        ptr: b"USD".as_ptr(),
                        len: 3,
                    },
                },
                collateral_asset: OpenPitStringView {
                    ptr: b"USD".as_ptr(),
                    len: 3,
                },
                average_entry_price: crate::param::OpenPitParamPriceOptional {
                    is_set: true,
                    value: OpenPitParamPrice(
                        Price::from_str("1").expect("price").to_decimal().into(),
                    ),
                },
                leverage: 10,
                mode: OpenPitParamPositionMode::Netting,
            },
            is_set: true,
        };

        let error = import_account_adjustment(&input).expect_err("must fail");
        assert!(error.contains("both balance and position operation"));
    }

    #[test]
    fn import_account_adjustment_rejects_incomplete_position_payload() {
        let input = OpenPitAccountAdjustment {
            balance_operation: OpenPitAccountAdjustmentBalanceOperationOptional::default(),
            position_operation: OpenPitAccountAdjustmentPositionOperationOptional {
                value: OpenPitAccountAdjustmentPositionOperation {
                    instrument: OpenPitInstrument::default(),
                    collateral_asset: OpenPitStringView::not_set(),
                    average_entry_price: crate::param::OpenPitParamPriceOptional::default(),
                    leverage: 10,
                    mode: OpenPitParamPositionMode::Hedged,
                },
                is_set: true,
            },
            amount: OpenPitAccountAdjustmentAmountOptional::default(),
            bounds: OpenPitAccountAdjustmentBoundsOptional::default(),
            user_data: std::ptr::null_mut(),
        };

        let error = import_account_adjustment(&input).expect_err("must fail");
        assert!(error.contains("position.instrument is not set"));
    }

    #[test]
    fn export_account_adjustment_produces_operation_specific_group() {
        let domain = RequestWithPayload::new(
            openpit_interop::AccountAdjustment {
                operation: AccountAdjustmentOperationAccess::Populated(
                    PopulatedAccountAdjustmentOperation::Position(
                        AccountAdjustmentPositionOperation {
                            instrument: Instrument::new(
                                Asset::new("SPX").expect("asset"),
                                Asset::new("USD").expect("asset"),
                            ),
                            collateral_asset: Asset::new("EUR").expect("asset"),
                            average_entry_price: Price::from_str("5").expect("price"),
                            mode: PositionMode::Hedged,
                            leverage: None,
                        },
                    ),
                ),
                amount: AccountAdjustmentAmountAccess::Absent,
                bounds: AccountAdjustmentBoundsAccess::Absent,
            },
            std::ptr::null_mut(),
        );

        let exported = export_account_adjustment(&domain);
        assert_eq!(
            exported.balance_operation,
            OpenPitAccountAdjustmentBalanceOperationOptional::default()
        );
        assert!(exported.position_operation.is_set);
        assert_eq!(
            exported
                .position_operation
                .value
                .instrument
                .underlying_asset
                .len,
            3
        );
        assert_eq!(
            exported
                .position_operation
                .value
                .instrument
                .settlement_asset
                .len,
            3
        );
        assert_eq!(exported.position_operation.value.collateral_asset.len, 3);
        assert!(exported.position_operation.value.average_entry_price.is_set);
        assert_eq!(
            exported.position_operation.value.mode,
            OpenPitParamPositionMode::Hedged
        );
    }

    #[test]
    fn import_export_account_adjustment_roundtrip() {
        let domain = RequestWithPayload::new(
            openpit_interop::AccountAdjustment {
                operation: AccountAdjustmentOperationAccess::Absent,
                amount: AccountAdjustmentAmountAccess::Populated(AccountAdjustmentAmount {
                    total: Some(AdjustmentAmount::Absolute(
                        PositionSize::from_str("4").expect("size"),
                    )),
                    reserved: None,
                    pending: Some(AdjustmentAmount::Delta(
                        PositionSize::from_str("1").expect("size"),
                    )),
                }),
                bounds: AccountAdjustmentBoundsAccess::Populated(AccountAdjustmentBounds {
                    total_upper: Some(PositionSize::from_str("8").expect("size")),
                    total_lower: None,
                    reserved_upper: None,
                    reserved_lower: None,
                    pending_upper: None,
                    pending_lower: Some(PositionSize::from_str("-2").expect("size")),
                }),
            },
            std::ptr::null_mut(),
        );

        let exported = export_account_adjustment(&domain);
        let imported = import_account_adjustment(&exported).expect("import");
        assert_eq!(imported, domain);
    }
}
