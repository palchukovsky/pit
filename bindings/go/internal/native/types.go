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

package native

/*
#include "openpit.h"
*/
import "C"

type AccountAdjustment = C.OpenPitAccountAdjustment
type AccountAdjustmentAmount = C.OpenPitAccountAdjustmentAmount
type AccountAdjustmentAmountOptional = C.OpenPitAccountAdjustmentAmountOptional
type AccountAdjustmentApplyStatus = C.OpenPitAccountAdjustmentApplyStatus
type AccountAdjustmentBalanceOperation = C.OpenPitAccountAdjustmentBalanceOperation
type AccountAdjustmentBalanceOperationOptional = C.OpenPitAccountAdjustmentBalanceOperationOptional
type AccountAdjustmentBatchError = *C.OpenPitAccountAdjustmentBatchError
type AccountAdjustmentBounds = C.OpenPitAccountAdjustmentBounds
type AccountAdjustmentBoundsOptional = C.OpenPitAccountAdjustmentBoundsOptional
type AccountAdjustmentContext = *C.OpenPitAccountAdjustmentContext
type AccountAdjustmentPolicy = *C.OpenPitAccountAdjustmentPolicy
type AccountAdjustmentPositionOperation = C.OpenPitAccountAdjustmentPositionOperation
type AccountAdjustmentPositionOperationOptional = C.OpenPitAccountAdjustmentPositionOperationOptional
type Engine = *C.OpenPitEngine
type EngineBuilder = *C.OpenPitEngineBuilder
type ExecutionReport = C.OpenPitExecutionReport
type ExecutionReportFill = C.OpenPitExecutionReportFill
type ExecutionReportFillOptional = C.OpenPitExecutionReportFillOptional
type ExecutionReportIsFinalOptional = C.OpenPitExecutionReportIsFinalOptional
type ExecutionReportOperation = C.OpenPitExecutionReportOperation
type ExecutionReportOperationOptional = C.OpenPitExecutionReportOperationOptional
type ExecutionReportPositionImpact = C.OpenPitExecutionReportPositionImpact
type ExecutionReportPositionImpactOptional = C.OpenPitExecutionReportPositionImpactOptional
type ExecutionReportTrade = C.OpenPitExecutionReportTrade
type ExecutionReportTradeOptional = C.OpenPitExecutionReportTradeOptional
type FinancialImpact = C.OpenPitFinancialImpact
type FinancialImpactOptional = C.OpenPitFinancialImpactOptional
type Instrument = C.OpenPitInstrument
type Mutations = *C.OpenPitMutations
type Order = C.OpenPitOrder
type OrderMargin = C.OpenPitOrderMargin
type OrderMarginOptional = C.OpenPitOrderMarginOptional
type OrderOperation = C.OpenPitOrderOperation
type OrderOperationOptional = C.OpenPitOrderOperationOptional
type OrderPosition = C.OpenPitOrderPosition
type OrderPositionOptional = C.OpenPitOrderPositionOptional
type ParamAccountID = C.OpenPitParamAccountId
type ParamAccountIDOptional = C.OpenPitParamAccountIdOptional
type ParamAdjustmentAmount = C.OpenPitParamAdjustmentAmount
type ParamAdjustmentAmountKind = C.OpenPitParamAdjustmentAmountKind
type TriBool = C.OpenPitTriBool
type ParamCashFlow = C.OpenPitParamCashFlow
type ParamCashFlowOptional = C.OpenPitParamCashFlowOptional
type ParamDecimal = C.OpenPitParamDecimal
type ParamFee = C.OpenPitParamFee
type ParamFeeOptional = C.OpenPitParamFeeOptional
type ParamLeverage = C.OpenPitParamLeverage
type ParamNotional = C.OpenPitParamNotional
type ParamNotionalOptional = C.OpenPitParamNotionalOptional
type ParamPnl = C.OpenPitParamPnl
type ParamPnlOptional = C.OpenPitParamPnlOptional
type ParamPositionEffect = C.OpenPitParamPositionEffect
type ParamPositionMode = C.OpenPitParamPositionMode
type ParamPositionSide = C.OpenPitParamPositionSide
type ParamPositionSize = C.OpenPitParamPositionSize
type ParamPositionSizeOptional = C.OpenPitParamPositionSizeOptional
type ParamPrice = C.OpenPitParamPrice
type ParamPriceOptional = C.OpenPitParamPriceOptional
type ParamQuantity = C.OpenPitParamQuantity
type ParamQuantityOptional = C.OpenPitParamQuantityOptional
type ParamError = *C.OpenPitParamError
type ParamErrorHandle = *C.OpenPitParamError
type ParamErrorCode = C.OpenPitParamErrorCode
type ParamRoundingStrategy = C.uint8_t
type ParamSide = C.OpenPitParamSide
type ParamTradeAmount = C.OpenPitParamTradeAmount
type ParamTradeAmountKind = C.OpenPitParamTradeAmountKind
type ParamVolume = C.OpenPitParamVolume
type ParamVolumeOptional = C.OpenPitParamVolumeOptional
type PretradeCheckPreTradeStartPolicy = *C.OpenPitPretradeCheckPreTradeStartPolicy
type PretradeContext = *C.OpenPitPretradeContext
type PretradePoliciesOrderSizeAccountAssetBarrier = C.OpenPitPretradePoliciesOrderSizeAccountAssetBarrier
type PretradePoliciesOrderSizeAssetBarrier = C.OpenPitPretradePoliciesOrderSizeAssetBarrier
type PretradePoliciesOrderSizeBrokerBarrier = C.OpenPitPretradePoliciesOrderSizeBrokerBarrier
type PretradePoliciesOrderSizeLimit = C.OpenPitPretradePoliciesOrderSizeLimit
type PretradePoliciesPnlBoundsAccountBarrier = C.OpenPitPretradePoliciesPnlBoundsAccountBarrier
type PretradePoliciesPnlBoundsBarrier = C.OpenPitPretradePoliciesPnlBoundsBarrier
type PretradePoliciesRateLimitAccountAssetBarrier = C.OpenPitPretradePoliciesRateLimitAccountAssetBarrier
type PretradePoliciesRateLimitAccountBarrier = C.OpenPitPretradePoliciesRateLimitAccountBarrier
type PretradePoliciesRateLimitAssetBarrier = C.OpenPitPretradePoliciesRateLimitAssetBarrier
type PretradePoliciesRateLimitBrokerBarrier = C.OpenPitPretradePoliciesRateLimitBrokerBarrier
type PretradePreTradeLock = C.OpenPitPretradePreTradeLock
type PretradePreTradePolicy = *C.OpenPitPretradePreTradePolicy
type PretradePreTradeRequest = *C.OpenPitPretradePreTradeRequest
type PretradePreTradeReservation = *C.OpenPitPretradePreTradeReservation
type Reject = C.OpenPitReject
type RejectCode = C.OpenPitRejectCode
type RejectList = *C.OpenPitRejectList
type RejectScope = C.OpenPitRejectScope
type SharedString = *C.OpenPitSharedString

const ParamLeverageNotSet = C.OPENPIT_PARAM_LEVERAGE_NOT_SET

const (
	ParamLeverageScale = C.OPENPIT_PARAM_LEVERAGE_SCALE
	ParamLeverageMin   = C.OPENPIT_PARAM_LEVERAGE_MIN
	ParamLeverageMax   = C.OPENPIT_PARAM_LEVERAGE_MAX
	ParamLeverageStep  = C.OPENPIT_PARAM_LEVERAGE_STEP
)

const (
	ParamSideNotSet = C.OpenPitParamSide_NotSet
	ParamSideBuy    = C.OpenPitParamSide_Buy
	ParamSideSell   = C.OpenPitParamSide_Sell
)

const (
	ParamPositionSideNotSet = C.OpenPitParamPositionSide_NotSet
	ParamPositionSideLong   = C.OpenPitParamPositionSide_Long
	ParamPositionSideShort  = C.OpenPitParamPositionSide_Short
)

const (
	ParamPositionModeNotSet  = C.OpenPitParamPositionMode_NotSet
	ParamPositionModeNetting = C.OpenPitParamPositionMode_Netting
	ParamPositionModeHedged  = C.OpenPitParamPositionMode_Hedged
)

const (
	ParamPositionEffectNotSet = C.OpenPitParamPositionEffect_NotSet
	ParamPositionEffectOpen   = C.OpenPitParamPositionEffect_Open
	ParamPositionEffectClose  = C.OpenPitParamPositionEffect_Close
)

const (
	ParamTradeAmountKindNotSet   = C.OpenPitParamTradeAmountKind_NotSet
	ParamTradeAmountKindQuantity = C.OpenPitParamTradeAmountKind_Quantity
	ParamTradeAmountKindVolume   = C.OpenPitParamTradeAmountKind_Volume
)

const (
	ParamRoundingStrategyMidpointNearestEven  = C.OpenPitParamRoundingStrategy_MidpointNearestEven
	ParamRoundingStrategyMidpointAwayFromZero = C.OpenPitParamRoundingStrategy_MidpointAwayFromZero
	ParamRoundingStrategyUp                   = C.OpenPitParamRoundingStrategy_Up
	ParamRoundingStrategyDown                 = C.OpenPitParamRoundingStrategy_Down
)

const (
	ParamRoundingStrategyDefault            = C.OPENPIT_PARAM_ROUNDING_STRATEGY_DEFAULT
	ParamRoundingStrategyBanker             = C.OPENPIT_PARAM_ROUNDING_STRATEGY_BANKER
	ParamRoundingStrategyConservativeProfit = C.OPENPIT_PARAM_ROUNDING_STRATEGY_CONSERVATIVE_PROFIT
	ParamRoundingStrategyConservativeLoss   = C.OPENPIT_PARAM_ROUNDING_STRATEGY_CONSERVATIVE_LOSS
)

const (
	ParamErrorCodeUnspecified     = C.OpenPitParamErrorCode_Unspecified
	ParamErrorCodeNegative        = C.OpenPitParamErrorCode_Negative
	ParamErrorCodeDivisionByZero  = C.OpenPitParamErrorCode_DivisionByZero
	ParamErrorCodeOverflow        = C.OpenPitParamErrorCode_Overflow
	ParamErrorCodeUnderflow       = C.OpenPitParamErrorCode_Underflow
	ParamErrorCodeInvalidFloat    = C.OpenPitParamErrorCode_InvalidFloat
	ParamErrorCodeInvalidFormat   = C.OpenPitParamErrorCode_InvalidFormat
	ParamErrorCodeInvalidPrice    = C.OpenPitParamErrorCode_InvalidPrice
	ParamErrorCodeInvalidLeverage = C.OpenPitParamErrorCode_InvalidLeverage
	ParamErrorCodeAssetEmpty      = C.OpenPitParamErrorCode_AssetEmpty
	ParamErrorCodeAccountIdEmpty  = C.OpenPitParamErrorCode_AccountIdEmpty
	ParamErrorCodeOther           = C.OpenPitParamErrorCode_Other
)

const (
	TriBoolNotSet = C.OpenPitTriBool_NotSet
	TriBoolFalse  = C.OpenPitTriBool_False
	TriBoolTrue   = C.OpenPitTriBool_True
)

const (
	ParamAdjustmentAmountKindNotSet   = C.OpenPitParamAdjustmentAmountKind_NotSet
	ParamAdjustmentAmountKindDelta    = C.OpenPitParamAdjustmentAmountKind_Delta
	ParamAdjustmentAmountKindAbsolute = C.OpenPitParamAdjustmentAmountKind_Absolute
)

const (
	RejectScopeOrder   = C.OpenPitRejectScope_Order
	RejectScopeAccount = C.OpenPitRejectScope_Account
)

const (
	RejectCodeMissingRequiredField        = C.OpenPitRejectCode_MissingRequiredField
	RejectCodeInvalidFieldFormat          = C.OpenPitRejectCode_InvalidFieldFormat
	RejectCodeInvalidFieldValue           = C.OpenPitRejectCode_InvalidFieldValue
	RejectCodeUnsupportedOrderType        = C.OpenPitRejectCode_UnsupportedOrderType
	RejectCodeUnsupportedTimeInForce      = C.OpenPitRejectCode_UnsupportedTimeInForce
	RejectCodeUnsupportedOrderAttribute   = C.OpenPitRejectCode_UnsupportedOrderAttribute
	RejectCodeDuplicateClientOrderID      = C.OpenPitRejectCode_DuplicateClientOrderId
	RejectCodeTooLateToEnter              = C.OpenPitRejectCode_TooLateToEnter
	RejectCodeExchangeClosed              = C.OpenPitRejectCode_ExchangeClosed
	RejectCodeUnknownInstrument           = C.OpenPitRejectCode_UnknownInstrument
	RejectCodeUnknownAccount              = C.OpenPitRejectCode_UnknownAccount
	RejectCodeUnknownVenue                = C.OpenPitRejectCode_UnknownVenue
	RejectCodeUnknownClearingAccount      = C.OpenPitRejectCode_UnknownClearingAccount
	RejectCodeUnknownCollateralAsset      = C.OpenPitRejectCode_UnknownCollateralAsset
	RejectCodeInsufficientFunds           = C.OpenPitRejectCode_InsufficientFunds
	RejectCodeInsufficientMargin          = C.OpenPitRejectCode_InsufficientMargin
	RejectCodeInsufficientPosition        = C.OpenPitRejectCode_InsufficientPosition
	RejectCodeCreditLimitExceeded         = C.OpenPitRejectCode_CreditLimitExceeded
	RejectCodeRiskLimitExceeded           = C.OpenPitRejectCode_RiskLimitExceeded
	RejectCodeOrderExceedsLimit           = C.OpenPitRejectCode_OrderExceedsLimit
	RejectCodeOrderQtyExceedsLimit        = C.OpenPitRejectCode_OrderQtyExceedsLimit
	RejectCodeOrderNotionalExceedsLimit   = C.OpenPitRejectCode_OrderNotionalExceedsLimit
	RejectCodePositionLimitExceeded       = C.OpenPitRejectCode_PositionLimitExceeded
	RejectCodeConcentrationLimitExceeded  = C.OpenPitRejectCode_ConcentrationLimitExceeded
	RejectCodeLeverageLimitExceeded       = C.OpenPitRejectCode_LeverageLimitExceeded
	RejectCodeRateLimitExceeded           = C.OpenPitRejectCode_RateLimitExceeded
	RejectCodePnlKillSwitchTriggered      = C.OpenPitRejectCode_PnlKillSwitchTriggered
	RejectCodeAccountBlocked              = C.OpenPitRejectCode_AccountBlocked
	RejectCodeAccountNotAuthorized        = C.OpenPitRejectCode_AccountNotAuthorized
	RejectCodeComplianceRestriction       = C.OpenPitRejectCode_ComplianceRestriction
	RejectCodeInstrumentRestricted        = C.OpenPitRejectCode_InstrumentRestricted
	RejectCodeJurisdictionRestriction     = C.OpenPitRejectCode_JurisdictionRestriction
	RejectCodeWashTradePrevention         = C.OpenPitRejectCode_WashTradePrevention
	RejectCodeSelfMatchPrevention         = C.OpenPitRejectCode_SelfMatchPrevention
	RejectCodeShortSaleRestriction        = C.OpenPitRejectCode_ShortSaleRestriction
	RejectCodeRiskConfigurationMissing    = C.OpenPitRejectCode_RiskConfigurationMissing
	RejectCodeReferenceDataUnavailable    = C.OpenPitRejectCode_ReferenceDataUnavailable
	RejectCodeOrderValueCalculationFailed = C.OpenPitRejectCode_OrderValueCalculationFailed
	RejectCodeSystemUnavailable           = C.OpenPitRejectCode_SystemUnavailable
	RejectCodeCustom                      = C.OpenPitRejectCode_Custom
	RejectCodeOther                       = C.OpenPitRejectCode_Other
)
