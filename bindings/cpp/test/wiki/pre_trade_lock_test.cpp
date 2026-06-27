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
// Please see https://openpit.dev and the OWNERS file for details.

// Source: Pre-Trade-Lock.md
//
// Compiling mirror of the C++ snippet published on the Pre-Trade-Lock wiki
// page. Each TEST runs the same user code shown in the corresponding C++
// subsection, wrapped only in the minimal engine / harness (setup + asserts)
// the snippet elides for readability. The published snippet body and the test
// body must stay in lock-step.

#include "openpit/account_adjustment.hpp"
#include "openpit/engine.hpp"
#include "openpit/model.hpp"
#include "openpit/param.hpp"
#include "openpit/pretrade/pretrade.hpp"

#include <gtest/gtest.h>
#include <openpit.h>

#include <cassert>
#include <optional>
#include <string>
#include <vector>

namespace {

//------------------------------------------------------------------------------
// Persisting and Restoring a Lock
//
// Mirrors the "Persisting and Restoring a Lock" example block: reserves a buy,
// serializes its lock to JSON, then restores the lock and feeds it back on the
// final fill so the held funds reconcile cleanly.

TEST(PreTradeLockWiki, PersistAndRestoreLockRoundTrip) {
  // Limit-only spot funds: the lock price is required to reconcile fills.
  openpit::EngineBuilder builder(openpit::SyncPolicy::None);
  openpit::pretrade::policies::SpotFundsPolicy{}.AddTo(builder);
  openpit::Engine engine = builder.Build();

  const openpit::param::AccountId accountId =
      openpit::param::AccountId::FromUint64(99224416);

  // Seed 10000 USD so the buy can be reserved.
  openpit::accountadjustment::AccountAdjustment seed;
  openpit::accountadjustment::BalanceOperation balanceOp;
  balanceOp.asset = "USD";
  seed.operation =
      openpit::accountadjustment::Operation::OfBalance(std::move(balanceOp));
  openpit::accountadjustment::Amount seedAmount;
  seedAmount.balance = openpit::param::AdjustmentAmount::OfAbsolute(
      openpit::param::PositionSize::FromString("10000"));
  seed.amount = std::move(seedAmount);

  const openpit::AdjustmentResult seedResult = engine.ApplyAccountAdjustment(
      accountId,
      std::vector<openpit::accountadjustment::AccountAdjustment>{seed});
  assert(seedResult.Passed());

  // Buy 10 AAPL @ 200 holds 2000 USD and records the lock price (200).
  openpit::model::Order order = openpit::model::Order::Limit(
      openpit::model::Instrument("AAPL", "USD"), accountId,
      openpit::model::Side::Buy,
      openpit::model::TradeAmount::OfQuantity(
          openpit::param::Quantity::FromString("10")),
      openpit::param::Price::FromString("200"));

  openpit::pretrade::ExecuteResult result = engine.ExecutePreTrade(order);
  assert(result.Passed());

  // Persist the lock with its built-in JSON serialization before committing.
  openpit::pretrade::PreTradeLock lock(
      openpit_pretrade_pre_trade_reservation_get_lock(
          result.reservation->Get()));
  const std::string payload = lock.ToJson();
  result.reservation->Commit();

  // --- After a process restart, rebuild the lock from your store. ---
  openpit::pretrade::PreTradeLock restored =
      openpit::pretrade::PreTradeLock::FromJson(payload);

  // The final fill must carry the restored lock so the policy reconciles the
  // 2000 USD it held against the real fill instead of blocking the account.
  // Keep the instrument alive so its string views remain valid for the call.
  const openpit::model::Instrument fillInstrument("AAPL", "USD");
  OpenPitExecutionReport raw{};
  raw.operation.is_set = true;
  raw.operation.value.instrument = fillInstrument.Raw();
  raw.operation.value.account_id.value = accountId.Raw();
  raw.operation.value.account_id.is_set = true;
  raw.operation.value.side = OpenPitParamSide_Buy;
  raw.fill.is_set = true;
  raw.fill.value.last_trade.is_set = true;
  raw.fill.value.last_trade.value.price =
      openpit::param::Price::FromString("200").Raw();
  raw.fill.value.last_trade.value.quantity =
      openpit::param::Quantity::FromString("10").Raw();
  raw.fill.value.leaves_quantity.is_set = true;
  raw.fill.value.leaves_quantity.value =
      openpit::param::Quantity::FromString("0").Raw();
  raw.fill.value.lock = restored.Get();
  raw.fill.value.is_final.is_set = true;
  raw.fill.value.is_final.value = true;

  OpenPitPretradeAccountBlockList* blocks = nullptr;
  OpenPitAccountAdjustmentOutcomeList* fillOutcomes = nullptr;
  OpenPitSharedString* error = nullptr;
  openpit_engine_apply_execution_report(engine.Get(), &raw, &blocks,
                                        &fillOutcomes, &error);

  // Harness assertions: the buy reserved successfully, the lock serialized and
  // restored cleanly, and the final fill produced no account blocks.
  EXPECT_TRUE(result.Passed());
  EXPECT_FALSE(payload.empty());
  EXPECT_FALSE(restored.IsEmpty());
  EXPECT_EQ(blocks, nullptr);

  if (blocks != nullptr) {
    openpit_pretrade_destroy_account_block_list(blocks);
  }
  if (fillOutcomes != nullptr) {
    openpit_destroy_account_adjustment_outcome_list(fillOutcomes);
  }
}

}  // namespace
