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

use openpit::param::{
    AccountId, AdjustmentAmount, Asset, Fee, Leverage, Pnl, PositionEffect, PositionMode,
    PositionSide, PositionSize, Price, Side, Trade, TradeAmount,
};
use openpit::{
    HasAccountAdjustmentBalanceAverageEntryPrice, HasAccountAdjustmentPending,
    HasAccountAdjustmentPendingLowerBound, HasAccountAdjustmentPendingUpperBound,
    HasAccountAdjustmentPositionLeverage, HasAccountAdjustmentReserved,
    HasAccountAdjustmentReservedLowerBound, HasAccountAdjustmentReservedUpperBound,
    HasAccountAdjustmentTotal, HasAccountAdjustmentTotalLowerBound,
    HasAccountAdjustmentTotalUpperBound, HasAccountId, HasAutoBorrow, HasAverageEntryPrice,
    HasBalanceAsset, HasClosePosition, HasCollateralAsset, HasExecutionReportIsFinal,
    HasExecutionReportLastTrade, HasExecutionReportPositionEffect, HasExecutionReportPositionSide,
    HasFee, HasInstrument, HasOrderCollateralAsset, HasOrderLeverage, HasOrderPositionSide,
    HasOrderPrice, HasPnl, HasPositionInstrument, HasPositionMode, HasReduceOnly, HasSide,
    HasTradeAmount, Instrument, RequestFieldAccessError,
};

use crate::{
    AccountAdjustmentAmountAccess, AccountAdjustmentBoundsAccess, AccountAdjustmentOperationAccess,
    ExecutionReportFillAccess, ExecutionReportOperationAccess, ExecutionReportPositionImpactAccess,
    FinancialImpactAccess, OrderMarginAccess, OrderOperationAccess, OrderPositionAccess,
};

/// Root order aggregate carrying access groups for all order fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Order {
    pub operation: OrderOperationAccess,
    pub position: OrderPositionAccess,
    pub margin: OrderMarginAccess,
}

impl Default for Order {
    fn default() -> Self {
        Self {
            operation: OrderOperationAccess::Absent,
            position: OrderPositionAccess::Absent,
            margin: OrderMarginAccess::Absent,
        }
    }
}

impl HasInstrument for Order {
    fn instrument(&self) -> Result<&Instrument, RequestFieldAccessError> {
        self.operation.instrument()
    }
}

impl HasSide for Order {
    fn side(&self) -> Result<Side, RequestFieldAccessError> {
        self.operation.side()
    }
}

impl HasAccountId for Order {
    fn account_id(&self) -> Result<AccountId, RequestFieldAccessError> {
        self.operation.account_id()
    }
}

impl HasTradeAmount for Order {
    fn trade_amount(&self) -> Result<TradeAmount, RequestFieldAccessError> {
        self.operation.trade_amount()
    }
}

impl HasOrderPrice for Order {
    fn price(&self) -> Result<Option<Price>, RequestFieldAccessError> {
        self.operation.price()
    }
}

impl HasOrderPositionSide for Order {
    fn position_side(&self) -> Result<Option<PositionSide>, RequestFieldAccessError> {
        self.position.position_side()
    }
}

impl HasReduceOnly for Order {
    fn reduce_only(&self) -> Result<bool, RequestFieldAccessError> {
        self.position.reduce_only()
    }
}

impl HasClosePosition for Order {
    fn close_position(&self) -> Result<bool, RequestFieldAccessError> {
        self.position.close_position()
    }
}

impl HasOrderLeverage for Order {
    fn leverage(&self) -> Result<Option<Leverage>, RequestFieldAccessError> {
        self.margin.leverage()
    }
}

impl HasOrderCollateralAsset for Order {
    fn collateral_asset(&self) -> Result<Option<&Asset>, RequestFieldAccessError> {
        self.margin.collateral_asset()
    }
}

impl HasAutoBorrow for Order {
    fn auto_borrow(&self) -> Result<bool, RequestFieldAccessError> {
        self.margin.auto_borrow()
    }
}

/// Root execution-report aggregate carrying access groups for all report fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionReport {
    pub operation: ExecutionReportOperationAccess,
    pub financial_impact: FinancialImpactAccess,
    pub fill: ExecutionReportFillAccess,
    pub position_impact: ExecutionReportPositionImpactAccess,
}

impl Default for ExecutionReport {
    fn default() -> Self {
        Self {
            operation: ExecutionReportOperationAccess::Absent,
            financial_impact: FinancialImpactAccess::Absent,
            fill: ExecutionReportFillAccess::Absent,
            position_impact: ExecutionReportPositionImpactAccess::Absent,
        }
    }
}

impl HasInstrument for ExecutionReport {
    fn instrument(&self) -> Result<&Instrument, RequestFieldAccessError> {
        self.operation.instrument()
    }
}

impl HasSide for ExecutionReport {
    fn side(&self) -> Result<Side, RequestFieldAccessError> {
        self.operation.side()
    }
}

impl HasAccountId for ExecutionReport {
    fn account_id(&self) -> Result<AccountId, RequestFieldAccessError> {
        self.operation.account_id()
    }
}

impl HasPnl for ExecutionReport {
    fn pnl(&self) -> Result<Pnl, RequestFieldAccessError> {
        self.financial_impact.pnl()
    }
}

impl HasFee for ExecutionReport {
    fn fee(&self) -> Result<Fee, RequestFieldAccessError> {
        self.financial_impact.fee()
    }
}

impl HasExecutionReportLastTrade for ExecutionReport {
    fn last_trade(&self) -> Result<Option<Trade>, RequestFieldAccessError> {
        self.fill.last_trade()
    }
}

impl HasExecutionReportIsFinal for ExecutionReport {
    fn is_final(&self) -> Result<bool, RequestFieldAccessError> {
        self.fill.is_final()
    }
}

impl HasExecutionReportPositionEffect for ExecutionReport {
    fn position_effect(&self) -> Result<Option<PositionEffect>, RequestFieldAccessError> {
        self.position_impact.position_effect()
    }
}

impl HasExecutionReportPositionSide for ExecutionReport {
    fn position_side(&self) -> Result<Option<PositionSide>, RequestFieldAccessError> {
        self.position_impact.position_side()
    }
}

/// Root account-adjustment aggregate carrying access groups for all adjustment fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountAdjustment {
    pub operation: AccountAdjustmentOperationAccess,
    pub amount: AccountAdjustmentAmountAccess,
    pub bounds: AccountAdjustmentBoundsAccess,
}

impl Default for AccountAdjustment {
    fn default() -> Self {
        Self {
            operation: AccountAdjustmentOperationAccess::Absent,
            amount: AccountAdjustmentAmountAccess::Absent,
            bounds: AccountAdjustmentBoundsAccess::Absent,
        }
    }
}

impl HasBalanceAsset for AccountAdjustment {
    fn balance_asset(&self) -> Result<&Asset, RequestFieldAccessError> {
        self.operation.balance_asset()
    }
}

impl HasAccountAdjustmentBalanceAverageEntryPrice for AccountAdjustment {
    fn balance_average_entry_price(&self) -> Result<Option<Price>, RequestFieldAccessError> {
        self.operation.balance_average_entry_price()
    }
}

impl HasPositionInstrument for AccountAdjustment {
    fn position_instrument(&self) -> Result<&Instrument, RequestFieldAccessError> {
        self.operation.position_instrument()
    }
}

impl HasCollateralAsset for AccountAdjustment {
    fn collateral_asset(&self) -> Result<&Asset, RequestFieldAccessError> {
        self.operation.collateral_asset()
    }
}

impl HasAverageEntryPrice for AccountAdjustment {
    fn average_entry_price(&self) -> Result<Price, RequestFieldAccessError> {
        self.operation.average_entry_price()
    }
}

impl HasPositionMode for AccountAdjustment {
    fn position_mode(&self) -> Result<PositionMode, RequestFieldAccessError> {
        self.operation.position_mode()
    }
}

impl HasAccountAdjustmentPositionLeverage for AccountAdjustment {
    fn position_leverage(&self) -> Result<Option<Leverage>, RequestFieldAccessError> {
        self.operation.position_leverage()
    }
}

impl HasAccountAdjustmentTotal for AccountAdjustment {
    fn total(&self) -> Result<Option<AdjustmentAmount>, RequestFieldAccessError> {
        self.amount.total()
    }
}

impl HasAccountAdjustmentReserved for AccountAdjustment {
    fn reserved(&self) -> Result<Option<AdjustmentAmount>, RequestFieldAccessError> {
        self.amount.reserved()
    }
}

impl HasAccountAdjustmentPending for AccountAdjustment {
    fn pending(&self) -> Result<Option<AdjustmentAmount>, RequestFieldAccessError> {
        self.amount.pending()
    }
}

impl HasAccountAdjustmentTotalUpperBound for AccountAdjustment {
    fn total_upper(&self) -> Result<Option<PositionSize>, RequestFieldAccessError> {
        self.bounds.total_upper()
    }
}

impl HasAccountAdjustmentTotalLowerBound for AccountAdjustment {
    fn total_lower(&self) -> Result<Option<PositionSize>, RequestFieldAccessError> {
        self.bounds.total_lower()
    }
}

impl HasAccountAdjustmentReservedUpperBound for AccountAdjustment {
    fn reserved_upper(&self) -> Result<Option<PositionSize>, RequestFieldAccessError> {
        self.bounds.reserved_upper()
    }
}

impl HasAccountAdjustmentReservedLowerBound for AccountAdjustment {
    fn reserved_lower(&self) -> Result<Option<PositionSize>, RequestFieldAccessError> {
        self.bounds.reserved_lower()
    }
}

impl HasAccountAdjustmentPendingUpperBound for AccountAdjustment {
    fn pending_upper(&self) -> Result<Option<PositionSize>, RequestFieldAccessError> {
        self.bounds.pending_upper()
    }
}

impl HasAccountAdjustmentPendingLowerBound for AccountAdjustment {
    fn pending_lower(&self) -> Result<Option<PositionSize>, RequestFieldAccessError> {
        self.bounds.pending_lower()
    }
}

/// Generic request wrapper pairing a domain root aggregate with a binding-layer payload.
///
/// The `request` field carries the validated domain values; the `payload` field
/// carries an opaque per-binding token whose meaning, lifetime, and thread-safety
/// are the caller's responsibility. Access to both is via the public fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestWithPayload<Request, Payload> {
    pub request: Request,
    pub payload: Payload,
}

impl<Request, Payload> RequestWithPayload<Request, Payload> {
    pub fn new(request: Request, payload: Payload) -> Self {
        Self { request, payload }
    }

    pub fn into_parts(self) -> (Request, Payload) {
        (self.request, self.payload)
    }
}

impl<Request: Default, Payload: Default> Default for RequestWithPayload<Request, Payload> {
    fn default() -> Self {
        Self {
            request: Request::default(),
            payload: Payload::default(),
        }
    }
}

impl<R: HasInstrument, P> HasInstrument for RequestWithPayload<R, P> {
    fn instrument(&self) -> Result<&Instrument, RequestFieldAccessError> {
        self.request.instrument()
    }
}

impl<R: HasSide, P> HasSide for RequestWithPayload<R, P> {
    fn side(&self) -> Result<Side, RequestFieldAccessError> {
        self.request.side()
    }
}

impl<R: HasAccountId, P> HasAccountId for RequestWithPayload<R, P> {
    fn account_id(&self) -> Result<AccountId, RequestFieldAccessError> {
        self.request.account_id()
    }
}

impl<R: HasTradeAmount, P> HasTradeAmount for RequestWithPayload<R, P> {
    fn trade_amount(&self) -> Result<TradeAmount, RequestFieldAccessError> {
        self.request.trade_amount()
    }
}

impl<R: HasOrderPrice, P> HasOrderPrice for RequestWithPayload<R, P> {
    fn price(&self) -> Result<Option<Price>, RequestFieldAccessError> {
        self.request.price()
    }
}

impl<R: HasOrderPositionSide, P> HasOrderPositionSide for RequestWithPayload<R, P> {
    fn position_side(&self) -> Result<Option<PositionSide>, RequestFieldAccessError> {
        self.request.position_side()
    }
}

impl<R: HasReduceOnly, P> HasReduceOnly for RequestWithPayload<R, P> {
    fn reduce_only(&self) -> Result<bool, RequestFieldAccessError> {
        self.request.reduce_only()
    }
}

impl<R: HasClosePosition, P> HasClosePosition for RequestWithPayload<R, P> {
    fn close_position(&self) -> Result<bool, RequestFieldAccessError> {
        self.request.close_position()
    }
}

impl<R: HasOrderLeverage, P> HasOrderLeverage for RequestWithPayload<R, P> {
    fn leverage(&self) -> Result<Option<Leverage>, RequestFieldAccessError> {
        self.request.leverage()
    }
}

impl<R: HasOrderCollateralAsset, P> HasOrderCollateralAsset for RequestWithPayload<R, P> {
    fn collateral_asset(&self) -> Result<Option<&Asset>, RequestFieldAccessError> {
        self.request.collateral_asset()
    }
}

impl<R: HasAutoBorrow, P> HasAutoBorrow for RequestWithPayload<R, P> {
    fn auto_borrow(&self) -> Result<bool, RequestFieldAccessError> {
        self.request.auto_borrow()
    }
}

impl<R: HasPnl, P> HasPnl for RequestWithPayload<R, P> {
    fn pnl(&self) -> Result<Pnl, RequestFieldAccessError> {
        self.request.pnl()
    }
}

impl<R: HasFee, P> HasFee for RequestWithPayload<R, P> {
    fn fee(&self) -> Result<Fee, RequestFieldAccessError> {
        self.request.fee()
    }
}

impl<R: HasExecutionReportLastTrade, P> HasExecutionReportLastTrade for RequestWithPayload<R, P> {
    fn last_trade(&self) -> Result<Option<Trade>, RequestFieldAccessError> {
        self.request.last_trade()
    }
}

impl<R: HasExecutionReportIsFinal, P> HasExecutionReportIsFinal for RequestWithPayload<R, P> {
    fn is_final(&self) -> Result<bool, RequestFieldAccessError> {
        self.request.is_final()
    }
}

impl<R: HasExecutionReportPositionEffect, P> HasExecutionReportPositionEffect
    for RequestWithPayload<R, P>
{
    fn position_effect(&self) -> Result<Option<PositionEffect>, RequestFieldAccessError> {
        self.request.position_effect()
    }
}

impl<R: HasExecutionReportPositionSide, P> HasExecutionReportPositionSide
    for RequestWithPayload<R, P>
{
    fn position_side(&self) -> Result<Option<PositionSide>, RequestFieldAccessError> {
        self.request.position_side()
    }
}

impl<R: HasBalanceAsset, P> HasBalanceAsset for RequestWithPayload<R, P> {
    fn balance_asset(&self) -> Result<&Asset, RequestFieldAccessError> {
        self.request.balance_asset()
    }
}

impl<R: HasAccountAdjustmentBalanceAverageEntryPrice, P>
    HasAccountAdjustmentBalanceAverageEntryPrice for RequestWithPayload<R, P>
{
    fn balance_average_entry_price(&self) -> Result<Option<Price>, RequestFieldAccessError> {
        self.request.balance_average_entry_price()
    }
}

impl<R: HasPositionInstrument, P> HasPositionInstrument for RequestWithPayload<R, P> {
    fn position_instrument(&self) -> Result<&Instrument, RequestFieldAccessError> {
        self.request.position_instrument()
    }
}

impl<R: HasCollateralAsset, P> HasCollateralAsset for RequestWithPayload<R, P> {
    fn collateral_asset(&self) -> Result<&Asset, RequestFieldAccessError> {
        self.request.collateral_asset()
    }
}

impl<R: HasAverageEntryPrice, P> HasAverageEntryPrice for RequestWithPayload<R, P> {
    fn average_entry_price(&self) -> Result<Price, RequestFieldAccessError> {
        self.request.average_entry_price()
    }
}

impl<R: HasPositionMode, P> HasPositionMode for RequestWithPayload<R, P> {
    fn position_mode(&self) -> Result<PositionMode, RequestFieldAccessError> {
        self.request.position_mode()
    }
}

impl<R: HasAccountAdjustmentPositionLeverage, P> HasAccountAdjustmentPositionLeverage
    for RequestWithPayload<R, P>
{
    fn position_leverage(&self) -> Result<Option<Leverage>, RequestFieldAccessError> {
        self.request.position_leverage()
    }
}

impl<R: HasAccountAdjustmentTotal, P> HasAccountAdjustmentTotal for RequestWithPayload<R, P> {
    fn total(&self) -> Result<Option<AdjustmentAmount>, RequestFieldAccessError> {
        self.request.total()
    }
}

impl<R: HasAccountAdjustmentReserved, P> HasAccountAdjustmentReserved for RequestWithPayload<R, P> {
    fn reserved(&self) -> Result<Option<AdjustmentAmount>, RequestFieldAccessError> {
        self.request.reserved()
    }
}

impl<R: HasAccountAdjustmentPending, P> HasAccountAdjustmentPending for RequestWithPayload<R, P> {
    fn pending(&self) -> Result<Option<AdjustmentAmount>, RequestFieldAccessError> {
        self.request.pending()
    }
}

impl<R: HasAccountAdjustmentTotalUpperBound, P> HasAccountAdjustmentTotalUpperBound
    for RequestWithPayload<R, P>
{
    fn total_upper(&self) -> Result<Option<PositionSize>, RequestFieldAccessError> {
        self.request.total_upper()
    }
}

impl<R: HasAccountAdjustmentTotalLowerBound, P> HasAccountAdjustmentTotalLowerBound
    for RequestWithPayload<R, P>
{
    fn total_lower(&self) -> Result<Option<PositionSize>, RequestFieldAccessError> {
        self.request.total_lower()
    }
}

impl<R: HasAccountAdjustmentReservedUpperBound, P> HasAccountAdjustmentReservedUpperBound
    for RequestWithPayload<R, P>
{
    fn reserved_upper(&self) -> Result<Option<PositionSize>, RequestFieldAccessError> {
        self.request.reserved_upper()
    }
}

impl<R: HasAccountAdjustmentReservedLowerBound, P> HasAccountAdjustmentReservedLowerBound
    for RequestWithPayload<R, P>
{
    fn reserved_lower(&self) -> Result<Option<PositionSize>, RequestFieldAccessError> {
        self.request.reserved_lower()
    }
}

impl<R: HasAccountAdjustmentPendingUpperBound, P> HasAccountAdjustmentPendingUpperBound
    for RequestWithPayload<R, P>
{
    fn pending_upper(&self) -> Result<Option<PositionSize>, RequestFieldAccessError> {
        self.request.pending_upper()
    }
}

impl<R: HasAccountAdjustmentPendingLowerBound, P> HasAccountAdjustmentPendingLowerBound
    for RequestWithPayload<R, P>
{
    fn pending_lower(&self) -> Result<Option<PositionSize>, RequestFieldAccessError> {
        self.request.pending_lower()
    }
}

#[cfg(test)]
mod tests {
    use openpit::param::{AccountId, Asset, Quantity, Side, TradeAmount};
    use openpit::{
        HasAccountAdjustmentTotal, HasAccountAdjustmentTotalUpperBound, HasAccountId,
        HasBalanceAsset, HasInstrument, HasOrderPrice, HasSide, HasTradeAmount, Instrument,
    };

    use super::*;
    use crate::{
        AccountAdjustmentAmountAccess, AccountAdjustmentBoundsAccess,
        AccountAdjustmentOperationAccess, OrderMarginAccess, OrderOperationAccess,
        OrderPositionAccess, PopulatedOrderOperation,
    };

    fn populated_order() -> Order {
        Order {
            operation: OrderOperationAccess::Populated(PopulatedOrderOperation {
                instrument: Some(Instrument::new(
                    Asset::new("BTC").expect("valid"),
                    Asset::new("USD").expect("valid"),
                )),
                account_id: Some(AccountId::from_u64(1)),
                side: Some(Side::Buy),
                trade_amount: Some(TradeAmount::Quantity(
                    Quantity::from_str("1").expect("valid"),
                )),
                price: None,
            }),
            position: OrderPositionAccess::Absent,
            margin: OrderMarginAccess::Absent,
        }
    }

    #[test]
    fn order_populated_operation_delegates_has_traits() {
        let order = populated_order();
        assert_eq!(
            order
                .instrument()
                .expect("instrument")
                .underlying_asset()
                .as_ref(),
            "BTC"
        );
        assert_eq!(order.side().expect("side"), Side::Buy);
        assert_eq!(
            order.account_id().expect("account_id"),
            AccountId::from_u64(1)
        );
        assert_eq!(
            order.trade_amount().expect("trade_amount"),
            TradeAmount::Quantity(Quantity::from_str("1").expect("valid"))
        );
        assert_eq!(order.price().expect("price"), None);
    }

    #[test]
    fn order_absent_groups_return_err_for_required_fields() {
        let order = populated_order();
        assert!(order.reduce_only().is_err());
        assert!(order.close_position().is_err());
        assert!(order.auto_borrow().is_err());
    }

    #[test]
    fn order_absent_position_returns_none_for_optional_side() {
        let order = populated_order();
        assert_eq!(order.position_side().expect("position_side"), None);
    }

    #[test]
    fn blanket_impl_on_request_with_payload_delegates_to_request() {
        let order = populated_order();
        let wrapped = RequestWithPayload::new(order.clone(), ());
        assert_eq!(wrapped.side().expect("side"), order.side().expect("side"));
        assert_eq!(
            wrapped
                .instrument()
                .expect("instrument")
                .underlying_asset()
                .as_ref(),
            "BTC"
        );
        assert_eq!(
            wrapped.trade_amount().expect("trade_amount"),
            order.trade_amount().expect("trade_amount")
        );
        assert!(wrapped.reduce_only().is_err());
    }

    #[test]
    fn request_with_payload_default_yields_absent_order_and_unit_payload() {
        let wrapped = RequestWithPayload::<Order, ()>::default();
        assert!(wrapped.instrument().is_err());
        assert!(wrapped.side().is_err());
        assert!(wrapped.reduce_only().is_err());
        assert_eq!(wrapped.payload, ());
    }

    #[test]
    fn blanket_impl_on_account_adjustment_forwards_absent_groups() {
        let adjustment = AccountAdjustment {
            operation: AccountAdjustmentOperationAccess::Absent,
            amount: AccountAdjustmentAmountAccess::Absent,
            bounds: AccountAdjustmentBoundsAccess::Absent,
        };
        let wrapped = RequestWithPayload::new(adjustment, ());
        assert!(wrapped.balance_asset().is_err());
        assert!(wrapped.total().is_err());
        assert!(wrapped.total_upper().is_err());
    }
}
