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

// Source: Balance-Reconciliation.md

#include "openpit/account_adjustment.hpp"
#include "openpit/engine.hpp"
#include "openpit/param.hpp"
#include "openpit/pretrade/pretrade.hpp"

#include <gtest/gtest.h>

#include <cassert>
#include <optional>
#include <vector>

namespace {

namespace aa = openpit::accountadjustment;
namespace param = openpit::param;
namespace policies = openpit::pretrade::policies;

// Builds a spot-funds engine with no-sync storage, mirroring the Python
// no_sync() harness used in the sibling snippet.
[[nodiscard]] openpit::Engine SpotFundsEngine() {
  openpit::EngineBuilder builder(openpit::SyncPolicy::None);
  builder.Add(policies::SpotFundsPolicy{});
  return builder.Build();
}

// Builds an absolute-balance adjustment for `asset` targeting `amount`.
[[nodiscard]] aa::AccountAdjustment Seed(const char* amount,
                                         const char* asset = "USD") {
  aa::AccountAdjustment adj;
  aa::BalanceOperation op;
  op.asset = asset;
  adj.operation = aa::Operation::OfBalance(op);
  aa::Amount amountGroup;
  amountGroup.balance = param::AdjustmentAmount::OfAbsolute(
      param::PositionSize::FromString(amount));
  adj.amount = amountGroup;
  return adj;
}

// ----------------------------------------------------------------------------
// Delta vs Absolute — mirrors the "Delta Versus Absolute" example block in
// Balance-Reconciliation.md.
//
// The engine is seeded twice for account 99224416 / USD.  The first seed sets
// available USD to 10000; both delta and absolute read 10000.  The second seed
// raises the target to 15000: delta is 5000 (the change) while absolute is
// 15000 (the new level).

TEST(BalanceReconciliation, DeltaVersusAbsolute) {
  openpit::Engine engine = SpotFundsEngine();
  const param::AccountId accountId = param::AccountId::FromUint64(99224416);

  // First seed: available USD goes from 0 to 10000.
  const openpit::AdjustmentResult firstResult = engine.ApplyAccountAdjustment(
      accountId, std::vector<aa::AccountAdjustment>{Seed("10000")});
  assert(firstResult.Passed());
  const std::vector<aa::Outcome>& firstOutcomes =
      firstResult.accountAdjustmentOutcomes;
  ASSERT_EQ(firstOutcomes.size(), 1u);
  ASSERT_TRUE(firstOutcomes[0].entry.balance.has_value());
  // delta is the change to add to your own ledger; absolute is just a snapshot.
  EXPECT_EQ(firstOutcomes[0].entry.balance->delta,
            param::PositionSize::FromString("10000"));
  EXPECT_EQ(firstOutcomes[0].entry.balance->absolute,
            param::PositionSize::FromString("10000"));

  // Second seed: available USD goes from 10000 to 15000.
  const openpit::AdjustmentResult secondResult = engine.ApplyAccountAdjustment(
      accountId, std::vector<aa::AccountAdjustment>{Seed("15000")});
  assert(secondResult.Passed());
  const std::vector<aa::Outcome>& secondOutcomes =
      secondResult.accountAdjustmentOutcomes;
  ASSERT_EQ(secondOutcomes.size(), 1u);
  ASSERT_TRUE(secondOutcomes[0].entry.balance.has_value());
  // delta is the change to add to your own ledger; absolute is just a snapshot.
  EXPECT_EQ(secondOutcomes[0].entry.balance->delta,
            param::PositionSize::FromString("5000"));
  EXPECT_EQ(secondOutcomes[0].entry.balance->absolute,
            param::PositionSize::FromString("15000"));
}

}  // namespace
