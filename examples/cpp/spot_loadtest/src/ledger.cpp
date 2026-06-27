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

#include "spot_loadtest/generator/ledger.hpp"

#include "spot_loadtest/decimal.hpp"
#include "spot_loadtest/generator/event.hpp"

#include <string>
#include <utility>

namespace spot_loadtest::generator {
namespace {

// The asset charged for an order and the charge amount (contract section 2.1,
// v1 subset: limit + quantity).
//   - Buy:  charge settlement, amount = q*p.
//   - Sell: charge underlying,  amount = q.
[[nodiscard]] std::pair<std::string, Decimal>
ChargeAsset(Side side, const std::string &underlying,
            const std::string &settlement, const Decimal &quantity,
            const Decimal &price) {
  if (side == Side::Buy) {
    return {settlement, ChargeForBuy(quantity, price)};
  }
  return {underlying, quantity}; // chargeForSell == q.
}

} // namespace

std::pair<Holdings, bool> Ledger::Get(const std::string &account,
                                      const std::string &asset) const {
  auto it = m_holdings.find(AssetKey{account, asset});
  if (it == m_holdings.end()) {
    return {Holdings{}, false};
  }
  return {it->second, true};
}

Decimal Ledger::Available(const std::string &account,
                          const std::string &asset) const {
  auto [h, ok] = Get(account, asset);
  return ok ? h.available : Decimal{};
}

void Ledger::Set(const std::string &account, const std::string &asset,
                 Holdings h) {
  const AssetKey key{account, asset};
  if (h.available.IsZero() && h.held.IsZero()) {
    m_holdings.erase(key);
    return;
  }
  m_holdings[key] = h;
}

PreTradeResult Ledger::PreTrade(const std::string &account, Side side,
                                const std::string &underlying,
                                const std::string &settlement,
                                const Decimal &quantity, const Decimal &price) {
  auto [asset, charge] =
      ChargeAsset(side, underlying, settlement, quantity, price);
  auto [cur, exists] = Get(account, asset);
  (void)exists;

  // Zero charge: accept without touching the slot (matches reserve_leg's
  // is_zero early return).
  if (charge.IsZero()) {
    PreTradeResult r;
    r.reason = RejectReason::None;
    r.chargeAsset = asset;
    r.chargeAmount = charge;
    r.postAvailable = cur.available;
    r.postHeld = cur.held;
    return r;
  }

  // try_hold: reject when charge > available (missing slot => available 0).
  if (charge > cur.available) {
    PreTradeResult r;
    r.reason = RejectReason::InsufficientFunds;
    r.chargeAsset = asset;
    r.chargeAmount = charge;
    r.postAvailable = cur.available;
    r.postHeld = cur.held;
    return r;
  }

  const Holdings next{cur.available - charge, cur.held + charge};
  Set(account, asset, next);
  PreTradeResult r;
  r.reason = RejectReason::None;
  r.chargeAsset = asset;
  r.chargeAmount = charge;
  r.postAvailable = next.available;
  r.postHeld = next.held;
  return r;
}

SettlementResult Ledger::SettleFullFill(const std::string &account, Side side,
                                        const std::string &underlying,
                                        const std::string &settlement,
                                        const Decimal &quantity,
                                        const Decimal &price) {
  const Decimal notional = ChargeForBuy(quantity, price); // q*p, exact.

  std::string heldAsset;
  std::string creditAsset;
  Decimal consume;
  Decimal credit;
  if (side == Side::Buy) {
    heldAsset = settlement;
    creditAsset = underlying;
    consume = notional;
    credit = quantity;
  } else {
    heldAsset = underlying;
    creditAsset = settlement;
    consume = quantity;
    credit = notional;
  }

  auto [held, ok] = Get(account, heldAsset);
  if (!ok || consume > held.held) {
    SettlementResult err;
    err.error = true; // would drive held negative; surfaced loudly by caller.
    return err;
  }
  const Holdings heldNext{held.available, held.held - consume};
  Set(account, heldAsset, heldNext);

  auto [creditCur, creditExists] = Get(account, creditAsset);
  (void)creditExists;
  const Holdings creditNext{creditCur.available + credit, creditCur.held};
  Set(account, creditAsset, creditNext);

  SettlementResult res;
  res.heldAsset = heldAsset;
  res.creditAsset = creditAsset;
  res.heldPost = heldNext;
  res.creditPost = creditNext;
  res.creditAmount = credit;
  return res;
}

FundingResult Ledger::ApplyFunding(const std::string &account,
                                   const std::string &asset, FundingKind kind,
                                   const Decimal &amount) {
  auto [cur, exists] = Get(account, asset);
  (void)exists;
  switch (kind) {
  case FundingKind::Absolute: {
    const Holdings next{amount, cur.held};
    Set(account, asset, next);
    FundingResult r;
    r.post = next;
    return r;
  }
  case FundingKind::Delta: {
    const Holdings next{cur.available + amount, cur.held};
    Set(account, asset, next);
    FundingResult r;
    r.post = next;
    return r;
  }
  }
  FundingResult r;
  r.reason = RejectReason::AccountAssetNotConfigured;
  r.rejected = true;
  r.post = cur;
  return r;
}

} // namespace spot_loadtest::generator
