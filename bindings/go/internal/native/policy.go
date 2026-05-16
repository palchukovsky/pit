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

import (
	"unsafe"
)

//------------------------------------------------------------------------------
// BuiltinRateLimitPolicy

func NewPretradePoliciesRateLimitBrokerBarrier(
	maxOrders uint,
	windowNanoseconds uint64,
) PretradePoliciesRateLimitBrokerBarrier {
	return PretradePoliciesRateLimitBrokerBarrier{
		max_orders:         C.size_t(maxOrders),
		window_nanoseconds: C.uint64_t(windowNanoseconds),
	}
}

func NewPretradePoliciesRateLimitAssetBarrier(
	maxOrders uint,
	windowNanoseconds uint64,
	settlementAsset string,
) PretradePoliciesRateLimitAssetBarrier {
	return PretradePoliciesRateLimitAssetBarrier{
		max_orders:         C.size_t(maxOrders),
		window_nanoseconds: C.uint64_t(windowNanoseconds),
		settlement_asset:   importString(settlementAsset),
	}
}

func NewPretradePoliciesRateLimitAccountBarrier(
	accountID ParamAccountID,
	maxOrders uint,
	windowNanoseconds uint64,
) PretradePoliciesRateLimitAccountBarrier {
	return PretradePoliciesRateLimitAccountBarrier{
		account_id:         accountID,
		max_orders:         C.size_t(maxOrders),
		window_nanoseconds: C.uint64_t(windowNanoseconds),
	}
}

func NewPretradePoliciesRateLimitAccountAssetBarrier(
	accountID ParamAccountID,
	maxOrders uint,
	windowNanoseconds uint64,
	settlementAsset string,
) PretradePoliciesRateLimitAccountAssetBarrier {
	return PretradePoliciesRateLimitAccountAssetBarrier{
		account_id:         accountID,
		max_orders:         C.size_t(maxOrders),
		window_nanoseconds: C.uint64_t(windowNanoseconds),
		settlement_asset:   importString(settlementAsset),
	}
}

//------------------------------------------------------------------------------
// BuiltinOrderSizeLimitPolicy

func NewPretradePoliciesOrderSizeLimit(
	maxQuantity ParamQuantity,
	maxNotional ParamVolume,
) PretradePoliciesOrderSizeLimit {
	return PretradePoliciesOrderSizeLimit{
		max_quantity: maxQuantity,
		max_notional: maxNotional,
	}
}

func NewPretradePoliciesOrderSizeBrokerBarrier(
	limit PretradePoliciesOrderSizeLimit,
) PretradePoliciesOrderSizeBrokerBarrier {
	return PretradePoliciesOrderSizeBrokerBarrier{limit: limit}
}

func NewPretradePoliciesOrderSizeAssetBarrier(
	limit PretradePoliciesOrderSizeLimit,
	settlementAsset string,
) PretradePoliciesOrderSizeAssetBarrier {
	return PretradePoliciesOrderSizeAssetBarrier{
		limit:            limit,
		settlement_asset: importString(settlementAsset),
	}
}

func NewPretradePoliciesOrderSizeAccountAssetBarrier(
	limit PretradePoliciesOrderSizeLimit,
	accountID ParamAccountID,
	settlementAsset string,
) PretradePoliciesOrderSizeAccountAssetBarrier {
	return PretradePoliciesOrderSizeAccountAssetBarrier{
		limit:            limit,
		account_id:       accountID,
		settlement_asset: importString(settlementAsset),
	}
}

//------------------------------------------------------------------------------
// BuiltinPnlBoundsKillswitchPolicy

func NewParamPnlOptional(value ParamPnl) ParamPnlOptional {
	var out ParamPnlOptional
	out.value = value
	out.is_set = true
	return out
}

func NewPretradePoliciesPnlBoundsBarrier(
	settlementAsset string,
	lowerBound ParamPnlOptional,
	upperBound ParamPnlOptional,
) PretradePoliciesPnlBoundsBarrier {
	return PretradePoliciesPnlBoundsBarrier{
		settlement_asset: importString(settlementAsset),
		lower_bound:      lowerBound,
		upper_bound:      upperBound,
	}
}

func NewPretradePoliciesPnlBoundsAccountBarrier(
	accountID ParamAccountID,
	settlementAsset string,
	lowerBound ParamPnlOptional,
	upperBound ParamPnlOptional,
	initialPnl ParamPnl,
) PretradePoliciesPnlBoundsAccountBarrier {
	return PretradePoliciesPnlBoundsAccountBarrier{
		account_id:       accountID,
		settlement_asset: importString(settlementAsset),
		lower_bound:      lowerBound,
		upper_bound:      upperBound,
		initial_pnl:      initialPnl,
	}
}

//------------------------------------------------------------------------------
// CheckPreTradeStartPolicy

func CreatePretradeCustomCheckPreTradeStartPolicy(
	name string,
	checkFnAddr unsafe.Pointer,
	applyExecutionReportFnAddr unsafe.Pointer,
	freeUserDataFnAddr unsafe.Pointer,
	userData unsafe.Pointer,
) (PretradeCheckPreTradeStartPolicy, error) {
	var outError SharedString
	p := C.openpit_create_pretrade_custom_check_pre_trade_start_policy(
		importString(name),
		*(*C.OpenPitPretradeCheckPreTradeStartPolicyCheckPreTradeStartFn)(checkFnAddr),
		*(*C.OpenPitPretradeCheckPreTradeStartPolicyApplyExecutionReportFn)(applyExecutionReportFnAddr),
		*(*C.OpenPitPretradeCheckPreTradeStartPolicyFreeUserDataFn)(freeUserDataFnAddr),
		userData,
		C.OpenPitOutError(&outError), //nolint:gocritic
	)
	if p == nil {
		return nil,
			consumeSharedStringAsError(
				outError,
				"openpit_create_pretrade_custom_check_pre_trade_start_policy failed",
			)
	}
	return p, nil
}

func DestroyPretradeCheckPreTradeStartPolicy(policy PretradeCheckPreTradeStartPolicy) {
	C.openpit_destroy_pretrade_check_pre_trade_start_policy(policy)
}

func PretradeCheckPreTradeStartPolicyGetName(
	policy PretradeCheckPreTradeStartPolicy,
) StringView {
	return newStringView(C.openpit_pretrade_check_pre_trade_start_policy_get_name(policy))
}

//------------------------------------------------------------------------------
// PreTradePolicy

func CreatePretradeCustomPreTradePolicy(
	name string,
	checkFnAddr unsafe.Pointer,
	applyFnAddr unsafe.Pointer,
	freeUserDataFnAddr unsafe.Pointer,
	userData unsafe.Pointer,
) (PretradePreTradePolicy, error) {
	var outError SharedString
	p := C.openpit_create_pretrade_custom_pre_trade_policy(
		importString(name),
		*(*C.OpenPitPretradePreTradePolicyCheckFn)(checkFnAddr),
		*(*C.OpenPitPretradePreTradePolicyApplyExecutionReportFn)(applyFnAddr),
		*(*C.OpenPitPretradePreTradePolicyFreeUserDataFn)(freeUserDataFnAddr),
		userData,
		C.OpenPitOutError(&outError), //nolint:gocritic
	)
	if p == nil {
		return nil,
			consumeSharedStringAsError(outError, "openpit_create_pretrade_custom_pre_trade_policy failed")
	}
	return p, nil
}

func DestroyPretradePreTradePolicy(policy PretradePreTradePolicy) {
	C.openpit_destroy_pretrade_pre_trade_policy(policy)
}

func PretradePreTradePolicyGetName(policy PretradePreTradePolicy) StringView {
	return newStringView(C.openpit_pretrade_pre_trade_policy_get_name(policy))
}

//------------------------------------------------------------------------------
// AccountAdjustmentPolicy

func CreateCustomAccountAdjustmentPolicy(
	name string,
	applyFnAddr unsafe.Pointer,
	freeUserDataFnAddr unsafe.Pointer,
	userData unsafe.Pointer,
) (AccountAdjustmentPolicy, error) {
	var outError SharedString
	p := C.openpit_create_custom_account_adjustment_policy(
		importString(name),
		*(*C.OpenPitAccountAdjustmentPolicyApplyFn)(applyFnAddr),
		*(*C.OpenPitAccountAdjustmentPolicyFreeUserDataFn)(freeUserDataFnAddr),
		userData,
		C.OpenPitOutError(&outError), //nolint:gocritic
	)
	if p == nil {
		return nil,
			consumeSharedStringAsError(outError, "openpit_create_custom_account_adjustment_policy failed")
	}
	return p, nil
}

func DestroyAccountAdjustmentPolicy(policy AccountAdjustmentPolicy) {
	C.openpit_destroy_account_adjustment_policy(policy)
}

func AccountAdjustmentPolicyGetName(policy AccountAdjustmentPolicy) StringView {
	return newStringView(C.openpit_account_adjustment_policy_get_name(policy))
}

//------------------------------------------------------------------------------
