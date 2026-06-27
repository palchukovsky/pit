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

// Source: Custom-Cpp-Types.md
//
// Compiling mirror of the C++ snippets published on the Custom C++ Types wiki
// page. The custom payload types, the typed policy, the adapter aliases, and
// the engine composition below are the exact user code from the page (modulo
// the minimal harness: an unqualified `Context` alias and the asserts that pin
// each outcome). When a snippet here changes, update the matching block in
// Custom-Cpp-Types.md and vice versa.

#include "openpit/adapters.hpp"
#include "openpit/engine.hpp"
#include "openpit/model.hpp"
#include "openpit/pretrade/pretrade.hpp"
#include "openpit/reject.hpp"

#include <gtest/gtest.h>

#include <optional>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>

namespace {

// The wiki snippets spell `Context` unqualified inside the main-stage hook, the
// same convention the SDK's own pretrade tests use.
using openpit::pretrade::Context;

//------------------------------------------------------------------------------
// Step 1 - define the custom payload types

// A desk order carries the standard model order plus a strategy tag the
// engine model does not own.
struct StrategyOrder : public openpit::Order {
  openpit::model::Order base;
  std::string strategyTag;

  // The engine consumes the embedded standard order; the policy still recovers
  // the typed StrategyOrder through the adapter.
  [[nodiscard]] OpenPitOrder EngineRaw() const noexcept override {
    return base.Raw();
  }
};

// A desk report carries the standard model report plus the venue execution id.
struct StrategyReport : public openpit::ExecutionReport {
  openpit::model::ExecutionReport base;
  std::string venueExecId;
};

//------------------------------------------------------------------------------
// Step 2 - write the typed policy

class StrategyTagPolicy {
 public:
  [[nodiscard]] std::string_view Name() const noexcept {
    return "StrategyTagPolicy";
  }

  // Start stage: reject a blocked strategy tag before the order enters the
  // pipeline. The typed StrategyOrder gives direct access to the project field.
  [[nodiscard]] std::optional<openpit::pretrade::Reject> CheckPreTradeStart(
      const StrategyOrder& order) const {
    if (order.strategyTag == "blocked") {
      return openpit::pretrade::Reject(
          std::string(Name()), openpit::pretrade::RejectScope::Order,
          openpit::pretrade::RejectCode::ComplianceRestriction,
          "strategy blocked", order.strategyTag);
    }
    return std::nullopt;
  }

  // Main stage: push a reject into the decision; an empty decision accepts.
  void PerformPreTradeCheck(const StrategyOrder& order, const Context& context,
                            openpit::pretrade::PolicyDecision& decision) const {
    static_cast<void>(context);
    if (order.strategyTag.empty()) {
      decision.Push(openpit::pretrade::Reject(
          std::string(Name()), openpit::pretrade::RejectScope::Order,
          openpit::pretrade::RejectCode::MissingRequiredField,
          "strategy tag is required", "strategyTag"));
    }
  }

  // Post-trade: return true to trip a kill-switch block; false otherwise.
  [[nodiscard]] bool ApplyExecutionReport(const StrategyReport& report) const {
    static_cast<void>(report);
    return false;
  }
};

//------------------------------------------------------------------------------
// Step 3 - bridge with an adapter (choose a cast mode)

// SafeSlow: the adapter uses dynamic_cast and produces a deterministic
// type-mismatch reject (start/main stage) or returns false (report stage) when
// the arriving payload is not the client type. Safe default at the boundary.
using StrategyMainAdapter = openpit::pretrade::PolicyAdapterWithSafeSlowArgType<
    StrategyTagPolicy, StrategyOrder, StrategyReport>;
using StrategyStartAdapter =
    openpit::pretrade::StartPolicyAdapterWithSafeSlowArgType<
        StrategyTagPolicy, StrategyOrder, StrategyReport>;

// UnsafeFast: direct static_cast, no runtime type check. A mismatched payload
// is undefined behavior, so this is only for closed, statically paired wiring.
using StrategyMainAdapterFast =
    openpit::pretrade::PolicyAdapterWithUnsafeFastArgType<
        StrategyTagPolicy, StrategyOrder, StrategyReport>;

//------------------------------------------------------------------------------
// Step 1: the custom payloads derive from the polymorphic bases and carry their
// project fields next to the embedded standard model groups.

TEST(WikiCustomCppTypes, DefineCustomPayloadTypes) {
  StrategyOrder order;
  order.strategyTag = "alpha";

  StrategyReport report;
  report.venueExecId = "EX-1";

  // Each derives from the adapter's polymorphic base.
  const openpit::Order& orderBase = order;
  const openpit::ExecutionReport& reportBase = report;
  static_cast<void>(orderBase);
  static_cast<void>(reportBase);

  EXPECT_EQ(order.strategyTag, "alpha");
  EXPECT_EQ(report.venueExecId, "EX-1");
}

//------------------------------------------------------------------------------
// Step 2 / Step 3: the SafeSlow start adapter dispatches to the typed policy on
// a matching payload and emits a deterministic reject on a foreign one.

TEST(WikiCustomCppTypes, StartAdapterTypedDispatchAndMismatch) {
  StrategyStartAdapter adapter{StrategyTagPolicy{}};

  StrategyOrder good;
  good.strategyTag = "alpha";
  EXPECT_FALSE(adapter.CheckPreTradeStart(good).has_value());

  StrategyOrder blocked;
  blocked.strategyTag = "blocked";
  const std::optional<openpit::pretrade::Reject> reject =
      adapter.CheckPreTradeStart(blocked);
  ASSERT_TRUE(reject.has_value());
  EXPECT_EQ(reject->code, openpit::pretrade::RejectCode::ComplianceRestriction);

  // A plain model order is NOT a StrategyOrder: SafeSlow yields a type-mismatch
  // reject rather than dispatching to the policy.
  openpit::model::Order foreign;
  const std::optional<openpit::pretrade::Reject> mismatch =
      adapter.CheckPreTradeStart(foreign);
  ASSERT_TRUE(mismatch.has_value());
  EXPECT_EQ(mismatch->code, openpit::pretrade::RejectCode::Other);
}

//------------------------------------------------------------------------------
// Step 3: the main-stage SafeSlow adapter pushes the policy's reject into the
// decision on a typed payload.

TEST(WikiCustomCppTypes, MainAdapterTypedDispatch) {
  StrategyMainAdapter adapter{StrategyTagPolicy{}};

  StrategyOrder missingTag;  // empty strategyTag -> policy rejects.
  const Context context(missingTag);

  openpit::pretrade::PolicyDecision decision;
  adapter.PerformPreTradeCheck(context, decision);
  ASSERT_TRUE(decision.IsRejected());
  EXPECT_EQ(decision.rejects.front().code,
            openpit::pretrade::RejectCode::MissingRequiredField);
}

//------------------------------------------------------------------------------
// Step 4 / Step 5: compose a typed engine and submit a custom order through the
// embedded standard model view.

TEST(WikiCustomCppTypes, ComposeEngineAndSubmit) {
  // --- begin wiki snippet (Step 4) ---
  openpit::EngineBuilder builder(openpit::SyncPolicy::None);

  // The CustomPolicy adopts the adapter (which owns the client policy) and
  // wires the detected hooks into the C ABI custom-policy vtable.
  openpit::pretrade::CustomPolicy<StrategyMainAdapter> mainPolicy(
      "StrategyTagPolicy", StrategyMainAdapter{StrategyTagPolicy{}});
  builder.Add(mainPolicy);

  const openpit::Engine engine = builder.Build();
  // --- end wiki snippet (Step 4) ---

  ASSERT_TRUE(static_cast<bool>(engine));

  // --- begin wiki snippet (Step 5) ---
  StrategyOrder order;
  openpit::model::OrderOperation op;
  op.instrument = openpit::model::Instrument("AAPL", "USD");
  op.accountId = ::openpit::param::AccountId::FromUint64(1);
  op.side = openpit::model::Side::Buy;
  op.tradeAmount = openpit::model::TradeAmount::OfQuantity(
      openpit::param::Quantity::FromString("1"));
  op.price = openpit::param::Price::FromString("100");
  order.base.operation = std::move(op);
  order.strategyTag = "alpha";

  // Submit the typed order; its EngineRaw() supplies the standard view that
  // drives the pipeline, while the policy still sees the typed StrategyOrder
  // through the adapter.
  openpit::pretrade::ExecuteResult result = engine.ExecutePreTrade(order);
  if (result.Passed()) {
    result.reservation->Commit();
  }
  // --- end wiki snippet (Step 5) ---

  EXPECT_TRUE(result.Passed());
}

// Keeps the UnsafeFast alias referenced so it is type-checked even though the
// happy-path tests above exercise the SafeSlow adapters.
static_assert(
    std::is_same_v<StrategyMainAdapterFast,
                   openpit::pretrade::PolicyAdapterWithUnsafeFastArgType<
                       StrategyTagPolicy, StrategyOrder, StrategyReport>>,
    "UnsafeFast alias must resolve to the UnsafeFast adapter specialization");

}  // namespace
