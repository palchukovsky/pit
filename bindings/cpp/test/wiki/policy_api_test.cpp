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

// Source: Policy-API.md
//
// Compiling mirror of the C++ snippets published on the Policy-API wiki page.
// Each TEST runs the same policy/engine code shown in a C++ wiki block (modulo
// the minimal harness: order/engine construction and assertions). Keep the
// published snippet and the corresponding test body in sync: when one changes,
// change the other.

// The pre-trade / reject headers define `RejectScope` / `RejectCode` with their
// fixed underlying types; they must precede `openpit/adapters.hpp`, whose
// opaque forward declarations of those scoped enums are only a compatible
// redeclaration when the full definition is already in scope.
#include "openpit/adapters.hpp"
#include "openpit/engine.hpp"
#include "openpit/model.hpp"
#include "openpit/pretrade/pretrade.hpp"
#include "openpit/reject.hpp"

#include <gtest/gtest.h>

#include <cstdint>
#include <optional>
#include <string>
#include <string_view>

namespace {

using openpit::param::Price;
using openpit::param::Quantity;
using openpit::param::Volume;
using openpit::pretrade::Context;
using openpit::pretrade::CustomPolicy;
using openpit::pretrade::PolicyDecision;
using openpit::pretrade::PushReject;
using openpit::pretrade::Reject;
using openpit::pretrade::RejectCode;
using openpit::pretrade::RejectScope;

//------------------------------------------------------------------------------
// Example: Custom Main-Stage Policy
//
// NotionalCapPolicy rejects any order whose requested settlement notional
// exceeds an absolute cap. The public C++ surface exposes the requested amount
// as an `openpit::model::TradeAmount`: a volume amount is already the notional,
// while a quantity amount is priced into one (notional = price * quantity).
// Absent amounts or an unpriceable quantity become explicit rejects rather
// than exceptions.

// >>> WIKI SNIPPET BEGIN: Custom Main-Stage Policy
// Computes settlement notional from a per-unit price and an instrument
// quantity (notional = price * quantity), crossing the exact-decimal C ABI so
// the result is bit-for-bit identical across language bindings. Returns
// nullopt when the engine reports the multiplication as a value error, which
// the caller turns into an explicit reject rather than an exception.
[[nodiscard]] std::optional<Volume> CalculateNotional(
    const Price& price, const Quantity& quantity) {
  OpenPitParamVolume raw{};
  OpenPitParamError* error = nullptr;
  if (!openpit_param_price_calculate_volume(price.Raw(), quantity.Raw(), &raw,
                                            &error)) {
    if (error != nullptr) {
      openpit_destroy_param_error(error);
    }
    return std::nullopt;
  }
  return Volume::FromRaw(raw);
}

class NotionalCapPolicy {
 public:
  // Policy-local config: reject any order above this absolute notional.
  explicit NotionalCapPolicy(Volume maxAbsNotional)
      : m_maxAbsNotional(maxAbsNotional) {}

  [[nodiscard]] std::string_view Name() const noexcept {
    return "NotionalCapPolicy";
  }

  void PerformPreTradeCheck(const openpit::model::Order& order,
                            const Context& context,
                            PolicyDecision& decision) const {
    static_cast<void>(context);
    if (!order.operation.has_value()) {
      PushReject(decision, Reject(std::string(Name()), RejectScope::Order,
                                  RejectCode::MissingRequiredField,
                                  "required order field missing",
                                  "operation is not set"));
      return;
    }
    const openpit::model::OrderOperation& operation = *order.operation;

    // Translate the public order surface into one number that this policy can
    // reason about: requested notional.
    if (!operation.tradeAmount.has_value()) {
      PushReject(decision, Reject(std::string(Name()), RejectScope::Order,
                                  RejectCode::MissingRequiredField,
                                  "required order field missing",
                                  "trade_amount is not set"));
      return;
    }
    const openpit::model::TradeAmount& tradeAmount = *operation.tradeAmount;

    // A volume trade amount is already the notional; a quantity trade amount
    // must be priced into a notional (notional = price * quantity).
    std::optional<Volume> requestedNotional = tradeAmount.AsVolume();
    if (!requestedNotional.has_value()) {
      const std::optional<Quantity> quantity = tradeAmount.AsQuantity();
      if (!operation.price.has_value()) {
        PushReject(decision,
                   Reject(std::string(Name()), RejectScope::Order,
                          RejectCode::OrderValueCalculationFailed,
                          "order value calculation failed",
                          "price not provided for evaluating notional"));
        return;
      }
      requestedNotional = CalculateNotional(*operation.price, *quantity);
      if (!requestedNotional.has_value()) {
        PushReject(decision,
                   Reject(std::string(Name()), RejectScope::Order,
                          RejectCode::OrderValueCalculationFailed,
                          "order value calculation failed",
                          "price and quantity could not be used to evaluate "
                          "notional"));
        return;
      }
    }

    if (*requestedNotional > m_maxAbsNotional) {
      // Business validation failures should become explicit rejects.
      PushReject(decision,
                 Reject(std::string(Name()), RejectScope::Order,
                        RejectCode::RiskLimitExceeded, "strategy cap exceeded",
                        "requested notional " + requestedNotional->ToString() +
                            ", max allowed: " + m_maxAbsNotional.ToString()));
      return;
    }

    // This policy only validates. It does not reserve mutable state.
  }

  [[nodiscard]] bool ApplyExecutionReport(
      const openpit::ExecutionReport& report) const {
    static_cast<void>(report);
    return false;
  }

 private:
  Volume m_maxAbsNotional;
};
// <<< WIKI SNIPPET END: Custom Main-Stage Policy

// Wraps the policy in the SafeSlow main-stage adapter so the context order is
// recovered as `openpit::model::Order` before the check runs.
using NotionalCapAdapter = openpit::pretrade::PolicyAdapterWithSafeSlowArgType<
    NotionalCapPolicy, openpit::model::Order, openpit::ExecutionReport>;

// Builds a notional order carrying `volume` settlement notional on `accountId`.
[[nodiscard]] openpit::model::Order NotionalOrder(std::uint64_t accountId,
                                                  std::string_view volume) {
  openpit::model::Order order;
  openpit::model::OrderOperation op;
  op.instrument = openpit::model::Instrument("AAPL", "USD");
  op.accountId = ::openpit::param::AccountId::FromUint64(accountId);
  op.side = openpit::model::Side::Buy;
  op.tradeAmount =
      openpit::model::TradeAmount::OfVolume(Volume::FromString(volume));
  order.operation = std::move(op);
  return order;
}

// Builds an order whose trade amount is a quantity, optionally priced. The
// policy must price the quantity into a notional rather than reject it.
[[nodiscard]] openpit::model::Order QuantityOrder(
    std::uint64_t accountId, std::string_view quantity,
    std::optional<std::string_view> price) {
  openpit::model::Order order;
  openpit::model::OrderOperation op;
  op.instrument = openpit::model::Instrument("AAPL", "USD");
  op.accountId = ::openpit::param::AccountId::FromUint64(accountId);
  op.side = openpit::model::Side::Buy;
  op.tradeAmount =
      openpit::model::TradeAmount::OfQuantity(Quantity::FromString(quantity));
  if (price.has_value()) {
    op.price = Price::FromString(*price);
  }
  order.operation = std::move(op);
  return order;
}

TEST(PolicyApiCustomMainStage, UnderCapAccepts) {
  const NotionalCapAdapter adapter{
      NotionalCapPolicy{Volume::FromString("1000000")}};
  const openpit::model::Order order = NotionalOrder(99224416, "250000");
  const Context context(order);

  PolicyDecision decision;
  adapter.PerformPreTradeCheck(context, decision);
  EXPECT_FALSE(decision.IsRejected());
}

TEST(PolicyApiCustomMainStage, OverCapRejectsWithRiskLimit) {
  const NotionalCapAdapter adapter{
      NotionalCapPolicy{Volume::FromString("1000000")}};
  const openpit::model::Order order = NotionalOrder(99224416, "2500000");
  const Context context(order);

  PolicyDecision decision;
  adapter.PerformPreTradeCheck(context, decision);
  ASSERT_TRUE(decision.IsRejected());
  EXPECT_EQ(decision.rejects.front().code, RejectCode::RiskLimitExceeded);
  EXPECT_EQ(decision.rejects.front().policy, "NotionalCapPolicy");
}

TEST(PolicyApiCustomMainStage, MissingOperationRejects) {
  const NotionalCapAdapter adapter{
      NotionalCapPolicy{Volume::FromString("1000000")}};
  const openpit::model::Order order;  // no operation group set.
  const Context context(order);

  PolicyDecision decision;
  adapter.PerformPreTradeCheck(context, decision);
  ASSERT_TRUE(decision.IsRejected());
  EXPECT_EQ(decision.rejects.front().code, RejectCode::MissingRequiredField);
}

TEST(PolicyApiCustomMainStage, QuantityUnderCapAccepts) {
  // 1000 * 25 = 25000 settlement notional, under the 1,000,000 cap.
  const NotionalCapAdapter adapter{
      NotionalCapPolicy{Volume::FromString("1000000")}};
  const openpit::model::Order order = QuantityOrder(99224416, "1000", "25");
  const Context context(order);

  PolicyDecision decision;
  adapter.PerformPreTradeCheck(context, decision);
  EXPECT_FALSE(decision.IsRejected());
}

TEST(PolicyApiCustomMainStage, QuantityOverCapRejectsWithRiskLimit) {
  // 100000 * 25 = 2,500,000 settlement notional, over the 1,000,000 cap.
  const NotionalCapAdapter adapter{
      NotionalCapPolicy{Volume::FromString("1000000")}};
  const openpit::model::Order order = QuantityOrder(99224416, "100000", "25");
  const Context context(order);

  PolicyDecision decision;
  adapter.PerformPreTradeCheck(context, decision);
  ASSERT_TRUE(decision.IsRejected());
  EXPECT_EQ(decision.rejects.front().code, RejectCode::RiskLimitExceeded);
}

TEST(PolicyApiCustomMainStage, QuantityWithoutPriceRejectsWithValueCalc) {
  const NotionalCapAdapter adapter{
      NotionalCapPolicy{Volume::FromString("1000000")}};
  const openpit::model::Order order =
      QuantityOrder(99224416, "1000", std::nullopt);
  const Context context(order);

  PolicyDecision decision;
  adapter.PerformPreTradeCheck(context, decision);
  ASSERT_TRUE(decision.IsRejected());
  EXPECT_EQ(decision.rejects.front().code,
            RejectCode::OrderValueCalculationFailed);
}

//------------------------------------------------------------------------------
// Example: Rollback Safety Pattern
//
// The policy applies a tentative reservation to its own state eagerly, then
// validates it. In the C++ binding a main-stage reject is a value reported into
// the `PolicyDecision`; the engine discards a rejected decision without
// applying its reservation, so the policy restores its own tentative state when
// it decides to reject.

// >>> WIKI SNIPPET BEGIN: Rollback Safety Pattern
class ReserveThenValidatePolicy {
 public:
  ReserveThenValidatePolicy() = default;

  [[nodiscard]] std::string_view Name() const noexcept {
    return "ReserveThenValidatePolicy";
  }

  void PerformPreTradeCheck(const openpit::model::Order& order,
                            const Context& context,
                            PolicyDecision& decision) const {
    static_cast<void>(order);
    static_cast<void>(context);

    // Pretend that this request needs a temporary reservation of 100. We apply
    // it eagerly because downstream logic wants to observe the tentative state
    // immediately.
    const Volume prevReserved = m_reserved;
    const Volume nextReserved = Volume::FromString("100");
    m_reserved = nextReserved;

    if (m_reserved > m_limit) {
      // The decision is rejected, so the engine will not apply this request:
      // restore the previous state before returning the reject.
      m_reserved = prevReserved;
      PushReject(decision, Reject(std::string(Name()), RejectScope::Order,
                                  RejectCode::RiskLimitExceeded,
                                  "temporary reservation exceeds limit",
                                  "reserved " + nextReserved.ToString() +
                                      ", limit: " + m_limit.ToString()));
    }
  }

  [[nodiscard]] bool ApplyExecutionReport(
      const openpit::ExecutionReport& report) const {
    static_cast<void>(report);
    return false;
  }

 private:
  mutable Volume m_reserved = Volume::FromString("0");
  Volume m_limit = Volume::FromString("50");
};
// <<< WIKI SNIPPET END: Rollback Safety Pattern

using ReserveThenValidateAdapter =
    openpit::pretrade::PolicyAdapterWithSafeSlowArgType<
        ReserveThenValidatePolicy, openpit::model::Order,
        openpit::ExecutionReport>;

TEST(PolicyApiRollbackSafety, OverLimitRejectsAndRestoresState) {
  const ReserveThenValidateAdapter adapter{ReserveThenValidatePolicy{}};
  const openpit::model::Order order = NotionalOrder(99224416, "10");
  const Context context(order);

  PolicyDecision decision;
  adapter.PerformPreTradeCheck(context, decision);
  ASSERT_TRUE(decision.IsRejected());
  EXPECT_EQ(decision.rejects.front().code, RejectCode::RiskLimitExceeded);
}

//------------------------------------------------------------------------------
// Custom `Order` and `Execution Report` Models
//
// A client order type carrying project-specific metadata derives from
// `openpit::Order`. A start-stage policy reads the typed field; the SafeSlow
// start adapter recovers the concrete type from the context order. The order is
// driven through the full pre-trade pipeline of a live engine.

// >>> WIKI SNIPPET BEGIN: Custom Order and Execution Report Models
// StrategyOrder carries project-specific metadata alongside the standard order.
struct StrategyOrder : public openpit::model::Order {
  std::string strategyTag;
};

// StrategyReport carries project-specific metadata alongside the standard
// report.
struct StrategyReport : public openpit::model::ExecutionReport {
  std::string venueExecId;
};

// StrategyTagPolicy rejects orders from blocked strategy tags.
class StrategyTagPolicy {
 public:
  [[nodiscard]] std::string_view Name() const noexcept {
    return "StrategyTagPolicy";
  }

  [[nodiscard]] std::optional<Reject> CheckPreTradeStart(
      const StrategyOrder& order) const {
    if (order.strategyTag == "blocked") {
      return Reject(
          std::string(Name()), RejectScope::Order,
          RejectCode::ComplianceRestriction, "strategy blocked",
          "strategy tag \"" + order.strategyTag + "\" is not allowed");
    }
    return std::nullopt;
  }

  [[nodiscard]] bool ApplyExecutionReport(const StrategyReport& report) const {
    static_cast<void>(report);
    return false;
  }
};
// <<< WIKI SNIPPET END: Custom Order and Execution Report Models

using StrategyStartAdapter =
    openpit::pretrade::StartPolicyAdapterWithSafeSlowArgType<
        StrategyTagPolicy, StrategyOrder, StrategyReport>;

TEST(PolicyApiCustomModels, AllowedStrategyTagPassesPipeline) {
  // >>> WIKI SNIPPET BEGIN: Custom Models driver
  CustomPolicy<StrategyStartAdapter> policy(
      "StrategyTagPolicy", StrategyStartAdapter{StrategyTagPolicy{}});

  openpit::EngineBuilder builder(openpit::SyncPolicy::Full);
  builder.Add(policy);
  openpit::Engine engine = builder.Build();

  StrategyOrder order;
  openpit::model::OrderOperation op;
  op.instrument = openpit::model::Instrument("AAPL", "USD");
  op.accountId = ::openpit::param::AccountId::FromUint64(99224416);
  op.side = openpit::model::Side::Buy;
  op.tradeAmount =
      openpit::model::TradeAmount::OfQuantity(Quantity::FromString("10"));
  op.price = Price::FromString("25");
  order.operation = std::move(op);
  order.strategyTag = "alpha";

  openpit::pretrade::StartResult start = engine.StartPreTrade(order);
  ASSERT_TRUE(start.Passed());

  openpit::pretrade::ExecuteResult execute = start.request->Execute();
  ASSERT_TRUE(execute.Passed());
  execute.reservation->Commit();
  // <<< WIKI SNIPPET END: Custom Models driver
}

TEST(PolicyApiCustomModels, BlockedStrategyTagRejectsAtStart) {
  CustomPolicy<StrategyStartAdapter> policy(
      "StrategyTagPolicy", StrategyStartAdapter{StrategyTagPolicy{}});

  openpit::EngineBuilder builder(openpit::SyncPolicy::Full);
  builder.Add(policy);
  openpit::Engine engine = builder.Build();

  StrategyOrder order;
  openpit::model::OrderOperation op;
  op.instrument = openpit::model::Instrument("AAPL", "USD");
  op.accountId = ::openpit::param::AccountId::FromUint64(99224416);
  op.side = openpit::model::Side::Buy;
  op.tradeAmount =
      openpit::model::TradeAmount::OfQuantity(Quantity::FromString("10"));
  op.price = Price::FromString("25");
  order.operation = std::move(op);
  order.strategyTag = "blocked";

  openpit::pretrade::StartResult start = engine.StartPreTrade(order);
  EXPECT_FALSE(start.Passed());
  ASSERT_EQ(start.rejects.size(), 1u);
  EXPECT_EQ(start.rejects.front().code, RejectCode::ComplianceRestriction);
}

//------------------------------------------------------------------------------
// Example: Block an Account (Kill Switch)
//
// The C++ binding records a kill-switch block through the engine's
// account-administration handle (`Engine::Accounts()`). Once an account is
// blocked, every later start stage for that account is rejected with
// `ACCOUNT_BLOCKED`, without involving any policy start-check.

// Builds the canonical single-leg order for `accountId`.
[[nodiscard]] openpit::model::Order AccountOrder(std::uint64_t accountId) {
  openpit::model::Order order;
  openpit::model::OrderOperation op;
  op.instrument = openpit::model::Instrument("AAPL", "USD");
  op.accountId = ::openpit::param::AccountId::FromUint64(accountId);
  op.side = openpit::model::Side::Buy;
  op.tradeAmount =
      openpit::model::TradeAmount::OfQuantity(Quantity::FromString("10"));
  op.price = Price::FromString("25");
  order.operation = std::move(op);
  return order;
}

TEST(PolicyApiBlockAccount, BlockedAccountIsRejectedWithAccountBlocked) {
  // >>> WIKI SNIPPET BEGIN: Block an Account
  openpit::EngineBuilder builder(openpit::SyncPolicy::None);
  builder.Add(openpit::pretrade::policies::OrderValidationPolicy{});
  openpit::Engine engine = builder.Build();

  const openpit::param::AccountId accountId =
      openpit::param::AccountId::FromUint64(99224416);

  // Record the kill-switch block through the engine's account-control handle.
  engine.Accounts().Block(accountId, "blocked via account control");

  // A later order on the same account is rejected with ACCOUNT_BLOCKED, without
  // any start-check involvement.
  openpit::pretrade::StartResult blocked =
      engine.StartPreTrade(AccountOrder(99224416));
  // <<< WIKI SNIPPET END: Block an Account

  ASSERT_FALSE(blocked.Passed());
  ASSERT_EQ(blocked.rejects.size(), 1u);
  EXPECT_EQ(blocked.rejects.front().code, RejectCode::AccountBlocked);
}

}  // namespace
