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

#pragma once

#include "spot_loadtest/generator/event.hpp"

#include "openpit/account_adjustment.hpp"
#include "openpit/account_id.hpp"
#include "openpit/accounts.hpp"
#include "openpit/asyncengine/typed.hpp"
#include "openpit/engine.hpp"
#include "openpit/model.hpp"
#include "openpit/param.hpp"
#include "openpit/pretrade/policies.hpp"
#include "openpit/pretrade/pre_trade_lock.hpp"
#include "openpit/reject.hpp"

#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

// Internal driver build helpers (mirror of build.go).
//
// event -> engine object mapping plus the engine driver, mirroring build.go
// exactly. The order/report/adjustment construction uses only real openpit::
// value types.
//
// LOCK-BEARING SETTLEMENT. The Go harness attaches a pre-trade lock to the
// execution-report fill (built from `pretrade.NewLockFromEntries`) so the
// spot-funds policy can resolve the held leg of a BUY fill at the reserved
// price. The public C++ `model::Fill::Raw()` deliberately leaves the fill lock
// null, so a plain `model::ExecutionReport` cannot carry a lock. To mirror the
// Go settlement faithfully through the public surface we use a custom report
// type
// (`ReportWithLock`) whose `Raw()` sets the fill lock pointer from a real
// `openpit::pretrade::PreTradeLock`, and a custom engine driver
// (`LockingEngineAdapter`) that mirrors `asyncengine::EngineAdapter` but
// applies such reports. Everything else (orders, adjustments) uses the stock
// model types. This keeps the methodology one-to-one with Go while using only
// real openpit:: symbols.

namespace spot_loadtest::driver::detail {

namespace ae = ::openpit::asyncengine;

// Maps a generator side to a model side.
[[nodiscard]] inline ::openpit::model::Side SideOf(generator::Side s) {
  switch (s) {
  case generator::Side::Buy:
    return ::openpit::model::Side::Buy;
  case generator::Side::Sell:
    return ::openpit::model::Side::Sell;
  }
  throw std::runtime_error("driver: unknown side");
}

// Builds the (underlying, settlement) instrument identity.
[[nodiscard]] inline ::openpit::model::Instrument
InstrumentOf(const std::string &underlying, const std::string &settlement) {
  return ::openpit::model::Instrument(underlying, settlement);
}

// Maps an OrderCheck event to a limit, quantity-denominated model::Order. The
// account id is the FNV-hashed string id, matching the Go binding.
[[nodiscard]] inline ::openpit::model::Order
BuildOrder(const generator::Event &ev,
           ::openpit::param::AccountId &outAccount) {
  outAccount = ::openpit::param::AccountId::FromString(ev.account);
  ::openpit::model::Order order;
  ::openpit::model::OrderOperation op;
  op.instrument = InstrumentOf(ev.underlying, ev.settlement);
  op.accountId = outAccount;
  op.side = SideOf(ev.side);
  op.tradeAmount = ::openpit::model::TradeAmount::OfQuantity(
      ::openpit::param::Quantity::FromString(ev.quantity.ToString()));
  op.price = ::openpit::param::Price::FromString(ev.price.ToString());
  order.operation = std::move(op);
  return order;
}

// A custom execution-report payload that mirrors model::ExecutionReport but
// attaches a pre-trade lock to the fill, exactly as Go's buildReport does. The
// lock pins the single reserved price under the default policy group so the
// spot-funds policy resolves a BUY fill's held leg.
//
// Move-only: it owns the PreTradeLock the Raw() view borrows.
class ReportWithLock {
public:
  ReportWithLock(::openpit::model::ExecutionReport report,
                 ::openpit::pretrade::PreTradeLock lock)
      : m_report(std::move(report)), m_lock(std::move(lock)) {}

  ReportWithLock(ReportWithLock &&) noexcept = default;
  ReportWithLock &operator=(ReportWithLock &&) noexcept = default;
  ReportWithLock(const ReportWithLock &) = delete;
  ReportWithLock &operator=(const ReportWithLock &) = delete;

  // Borrows this object's storage; valid only while it stays alive and
  // unchanged. The fill lock pointer is set from the owned PreTradeLock.
  [[nodiscard]] OpenPitExecutionReport Raw() const noexcept {
    OpenPitExecutionReport raw = m_report.Raw();
    if (raw.fill.is_set) {
      raw.fill.value.lock = m_lock.Get();
    }
    return raw;
  }

private:
  ::openpit::model::ExecutionReport m_report;
  ::openpit::pretrade::PreTradeLock m_lock;
};

// Maps a Settlement event to a full-fill (leaves = 0, is_final = true) report
// plus the matching reserved-price lock under the default policy group.
[[nodiscard]] inline ReportWithLock
BuildReport(const generator::Event &ev,
            ::openpit::param::AccountId &outAccount) {
  outAccount = ::openpit::param::AccountId::FromString(ev.account);
  const ::openpit::param::Price price =
      ::openpit::param::Price::FromString(ev.price.ToString());
  const ::openpit::param::Quantity qty =
      ::openpit::param::Quantity::FromString(ev.quantity.ToString());
  const ::openpit::param::Quantity leaves =
      ::openpit::param::Quantity::FromString("0");
  const ::openpit::param::Fee fee = ::openpit::param::Fee::FromString("0");
  const ::openpit::param::Pnl pnl = ::openpit::param::Pnl::FromString("0");

  ::openpit::model::ExecutionReport report;
  ::openpit::model::ExecutionReportOperation op;
  op.instrument = InstrumentOf(ev.underlying, ev.settlement);
  op.accountId = outAccount;
  op.side = SideOf(ev.side);
  report.operation = std::move(op);

  ::openpit::model::FinancialImpact fin;
  fin.pnl = pnl;
  fin.fee = fee;
  report.financialImpact = std::move(fin);

  ::openpit::model::Fill fill;
  fill.lastTrade = ::openpit::model::Trade(price, qty);
  fill.leavesQuantity = leaves;
  fill.isFinal = true;
  report.fill = std::move(fill);

  // The fill lock ties the report back to the reservation the order committed:
  // one entry under the default policy group at the SAME price the order
  // reserved at (the analogue of Go's NewLockFromEntries).
  ::openpit::pretrade::PreTradeLock lock;
  lock.Push(::openpit::param::DefaultPolicyGroupId, price);

  return ReportWithLock(std::move(report), std::move(lock));
}

// Maps a Funding event to a balance-operation adjustment on the funded asset's
// available leg (held is never touched), Absolute or Delta per the event's
// kind.
[[nodiscard]] inline ::openpit::accountadjustment::AccountAdjustment
BuildAdjustment(const generator::Event &ev,
                ::openpit::param::AccountId &outAccount) {
  outAccount = ::openpit::param::AccountId::FromString(ev.account);
  const ::openpit::param::PositionSize amount =
      ::openpit::param::PositionSize::FromString(ev.fundingAmount.ToString());

  ::openpit::param::AdjustmentAmount balance =
      ev.FundingIsDelta()
          ? ::openpit::param::AdjustmentAmount::OfDelta(amount)
          : ::openpit::param::AdjustmentAmount::OfAbsolute(amount);

  ::openpit::accountadjustment::BalanceOperation balanceOp;
  balanceOp.asset = ev.fundingAsset;

  ::openpit::accountadjustment::Amount amountGroup;
  amountGroup.balance = balance;

  ::openpit::accountadjustment::AccountAdjustment adj;
  adj.operation =
      ::openpit::accountadjustment::Operation::OfBalance(std::move(balanceOp));
  adj.amount = std::move(amountGroup);
  return adj;
}

// A zero-value adjustment on a probe account (the harness self-overhead probe).
[[nodiscard]] inline ::openpit::accountadjustment::AccountAdjustment
BuildProbeAdjustment() {
  const ::openpit::param::PositionSize zero =
      ::openpit::param::PositionSize::FromString("0");
  ::openpit::accountadjustment::BalanceOperation balanceOp;
  balanceOp.asset = "USD";
  ::openpit::accountadjustment::Amount amountGroup;
  amountGroup.balance = ::openpit::param::AdjustmentAmount::OfDelta(zero);
  ::openpit::accountadjustment::AccountAdjustment adj;
  adj.operation =
      ::openpit::accountadjustment::Operation::OfBalance(std::move(balanceOp));
  adj.amount = std::move(amountGroup);
  return adj;
}

// An engine driver mirroring asyncengine::EngineAdapter but routing settlement
// through ReportWithLock so the lock reaches the policy. It satisfies the
// TypedAsyncEngine driver seam (the five members) used by the harness:
// ExecutePreTrade, ApplyExecutionReport, ApplyAccountAdjustment (plus the
// unused StartPreTrade / Accounts to complete the seam).
class LockingEngineAdapter {
public:
  explicit LockingEngineAdapter(const ::openpit::Engine &engine) noexcept
      : m_engine(&engine) {}

  [[nodiscard]] ::openpit::pretrade::StartResult
  StartPreTrade(const ::openpit::model::Order &order) const {
    return m_engine->StartPreTrade(order);
  }

  [[nodiscard]] ::openpit::pretrade::ExecuteResult
  ExecutePreTrade(const ::openpit::model::Order &order) const {
    return m_engine->ExecutePreTrade(order);
  }

  // Applies a report carrying a fill lock, so a BUY fill's held leg resolves.
  // The public `Engine::ApplyExecutionReport` takes a `model::ExecutionReport`
  // whose `Fill::Raw()` always nulls the lock, so it cannot carry one; we apply
  // the lock-bearing raw view through the engine's C ABI entry point (the same
  // symbol `Engine::ApplyExecutionReport` uses internally) and decode the
  // result with the binding's public value types, so the settlement path stays
  // bit-identical to the synchronous engine while preserving the lock.
  [[nodiscard]] ::openpit::PostTradeResult
  ApplyExecutionReport(const ReportWithLock &report) const {
    return ApplyRaw(report.Raw());
  }

  template <typename Adjustment>
  [[nodiscard]] ::openpit::AdjustmentResult
  ApplyAccountAdjustment(::openpit::param::AccountId accountId,
                         const std::vector<Adjustment> &adjustments) const {
    return m_engine->ApplyAccountAdjustment(accountId, adjustments);
  }

  [[nodiscard]] ::openpit::accounts::Accounts Accounts() const noexcept {
    return m_engine->Accounts();
  }

private:
  // Applies a raw execution-report view (with its fill lock set) through the
  // engine's C ABI entry point — the identical call
  // `Engine::ApplyExecutionReport` makes — then decodes the result with the
  // binding's public value types
  // (`accounts::AccountBlock`, `accountadjustment::OutcomeList`). This is the
  // only way to apply a lock-bearing report through the public surface, because
  // `model::Fill::Raw()` deliberately nulls the lock.
  [[nodiscard]] ::openpit::PostTradeResult
  ApplyRaw(OpenPitExecutionReport raw) const {
    OpenPitPretradeAccountBlockList *blocks = nullptr;
    OpenPitAccountAdjustmentOutcomeList *outcomes = nullptr;
    OpenPitSharedString *error = nullptr;
    if (!openpit_engine_apply_execution_report(m_engine->Get(), &raw, &blocks,
                                               &outcomes, &error)) {
      ::openpit::detail::ThrowFromSharedString(
          error, "openpit_engine_apply_execution_report failed");
    }
    ::openpit::PostTradeResult out;
    if (blocks != nullptr) {
      const std::size_t count = openpit_pretrade_account_block_list_len(blocks);
      out.accountBlocks.reserve(count);
      for (std::size_t i = 0; i < count; ++i) {
        OpenPitPretradeAccountBlock block{};
        if (openpit_pretrade_account_block_list_get(blocks, i, &block)) {
          out.accountBlocks.push_back(
              ::openpit::accounts::AccountBlock::FromRaw(block));
        }
      }
      openpit_pretrade_destroy_account_block_list(blocks);
    }
    const ::openpit::accountadjustment::OutcomeList outcomeList(outcomes);
    out.accountAdjustmentOutcomes = outcomeList.ToVector();
    return out;
  }

  const ::openpit::Engine *m_engine;
};

} // namespace spot_loadtest::driver::detail
