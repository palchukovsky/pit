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

// Source: Account-Adjustments.md
//
// Compiling mirror of the C++ snippets published on the Account-Adjustments
// wiki page. Each TEST runs the same user code shown in a wiki code block
// (modulo the minimal harness: engine setup is the snippet's own, asserts wrap
// the published assertions). Keep the bodies in sync with the published
// snippets whenever either side changes.

#include "openpit/account_adjustment.hpp"
#include "openpit/engine.hpp"
#include "openpit/model.hpp"
#include "openpit/param.hpp"
#include "openpit/pretrade/policies.hpp"

#include <gtest/gtest.h>

#include <cassert>
#include <cstdint>
#include <map>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace {

// Caller-side cumulative limit: the maximum absolute balance any single asset
// may reach across the adjustments seen so far. Money value types are opaque
// and carry no arithmetic, so the running totals live in a host-side ledger
// keyed by asset; the engine still receives exact `param` absolute values.
class CumulativeLimitPolicy {
 public:
  explicit CumulativeLimitPolicy(std::int64_t maxCumulative)
      : m_maxCumulative(maxCumulative) {}

  // Returns true when `asset` may be set to `absoluteUnits`; records the new
  // total only when it stays within the limit, so a rejected screen leaves the
  // ledger untouched (rollback by absolute value is safe here).
  [[nodiscard]] bool Admit(const std::string& asset,
                           std::int64_t absoluteUnits) {
    if (absoluteUnits > m_maxCumulative) {
      return false;
    }
    m_totals[asset] = absoluteUnits;
    return true;
  }

 private:
  std::int64_t m_maxCumulative;
  std::map<std::string, std::int64_t> m_totals;
};

// Mirrors the "Example: Balance Limit Policy" C++ block. A caller-side
// cumulative limit screens the prospective USD total before the engine call;
// the atomic batch supplies the rollback, so on accept the returned optional is
// empty and no engine state is left dirty.
TEST(AccountAdjustmentsWiki, CumulativeBalanceLimitScreensThenApplies) {
  namespace aa = openpit::accountadjustment;
  namespace param = openpit::param;
  namespace policies = openpit::pretrade::policies;

  // Build one batch that tops a USD cash balance up to an absolute value.
  const param::AccountId accountId = param::AccountId::FromUint64(99224416);

  CumulativeLimitPolicy limit(/*maxCumulative=*/1'000'000);

  aa::AccountAdjustment cashAdj;
  {
    aa::BalanceOperation balance;
    balance.asset = "USD";
    cashAdj.operation = aa::Operation::OfBalance(std::move(balance));
    aa::Amount amount;
    amount.balance = param::AdjustmentAmount::OfAbsolute(
        param::PositionSize::FromString("10000"));
    cashAdj.amount = std::move(amount);
  }

  // Screen the prospective total before touching the engine.
  assert(limit.Admit("USD", 10000));

  const std::vector<aa::AccountAdjustment> adjustments{std::move(cashAdj)};

  openpit::EngineBuilder builder(openpit::SyncPolicy::None);
  builder.Add(policies::OrderValidationPolicy{});
  const openpit::Engine engine = builder.Build();

  // The atomic batch is the rollback: on reject no engine state changes; on
  // accept the result passes.
  const openpit::AdjustmentResult result =
      engine.ApplyAccountAdjustment(accountId, adjustments);
  assert(result.Passed());

  EXPECT_TRUE(result.Passed());
}

// Mirrors the "Examples" mixed balance/position batch block. Builds one batch
// that sets a USD cash balance and an SPX/USD hedged position by absolute
// value, then applies it as a single atomic engine call and asserts acceptance.
TEST(AccountAdjustmentsWiki, MixedBalanceAndPositionBatchApplies) {
  namespace aa = openpit::accountadjustment;
  namespace param = openpit::param;
  namespace policies = openpit::pretrade::policies;

  // Build one batch that mixes balance and position adjustments.
  const param::AccountId accountId = param::AccountId::FromUint64(99224416);

  aa::AccountAdjustment cashAdj;
  {
    aa::BalanceOperation balance;
    balance.asset = "USD";
    cashAdj.operation = aa::Operation::OfBalance(std::move(balance));
    aa::Amount amount;
    amount.balance = param::AdjustmentAmount::OfAbsolute(
        param::PositionSize::FromString("10000"));
    cashAdj.amount = std::move(amount);
  }

  aa::AccountAdjustment posAdj;
  {
    aa::PositionOperation position;
    position.instrument = openpit::model::Instrument("SPX", "USD");
    position.collateralAsset = "USD";
    position.averageEntryPrice = param::Price::FromString("95000");
    position.mode = openpit::model::PositionMode::Hedged;
    posAdj.operation = aa::Operation::OfPosition(std::move(position));
    aa::Amount amount;
    amount.balance = param::AdjustmentAmount::OfAbsolute(
        param::PositionSize::FromString("-3"));
    posAdj.amount = std::move(amount);
  }

  const std::vector<aa::AccountAdjustment> adjustments{std::move(cashAdj),
                                                       std::move(posAdj)};

  // The engine validates the whole batch atomically.
  openpit::EngineBuilder builder(openpit::SyncPolicy::None);
  builder.Add(policies::OrderValidationPolicy{});
  const openpit::Engine engine = builder.Build();

  // On accept the result passes and carries the per-asset account-adjustment
  // outcomes.
  const openpit::AdjustmentResult result =
      engine.ApplyAccountAdjustment(accountId, adjustments);
  assert(result.Passed());

  EXPECT_TRUE(result.Passed());
}

}  // namespace
