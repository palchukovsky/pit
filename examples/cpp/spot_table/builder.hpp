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

// translators. Each helper converts one validated table row into the
// corresponding `openpit` value type the engine accepts.
//
// back to the matching ORDER's reservation under the spot policy's default
// group. The C++ `model::ExecutionReport::Raw()` always nulls the fill lock, so
// `FillReport` owns the execution report together with the `PreTradeLock` and
// emits the raw C view with the lock pointer patched in - the one place this

#include "openpit/account_adjustment.hpp"
#include "openpit/account_id.hpp"
#include "openpit/marketdata.hpp"
#include "openpit/model.hpp"
#include "openpit/param.hpp"
#include "openpit/pretrade/pre_trade_lock.hpp"

#include <openpit.h>

#include <memory>
#include <stdexcept>
#include <string>

#include "marketdata.hpp"
#include "table.hpp"

namespace spot_table {

// Thrown on any row -> payload translation failure (a malformed id, asset,
// builder helpers.
class BuildError : public std::runtime_error {
public:
  explicit BuildError(const std::string &message)
      : std::runtime_error(message) {}
};

// Converts a free-form table account string to a stable engine-side AccountId
// `accountID`.
[[nodiscard]] openpit::param::AccountId AccountIdOf(const std::string &s);

// Converts a free-form table group label to a stable engine-side
// `accountGroupID`.
[[nodiscard]] openpit::param::AccountGroupId
AccountGroupIdOf(const std::string &s);

// Turns "BASE/QUOTE" into an engine Instrument. Throws `BuildError` on a
[[nodiscard]] openpit::model::Instrument ParseInstrument(const std::string &s);

// `parseSide`.
[[nodiscard]] openpit::model::Side ParseSide(const std::string &s);

// Turns a SEED row into an AccountAdjustment that the spot policy accepts as an
// absolute starting balance for the asset. Throws `BuildError` on a malformed
[[nodiscard]] openpit::accountadjustment::AccountAdjustment
BuildSeedAdjustment(const Row &row);

// Turns an ORDER row into a `model::Order`. Empty price means market order; the
// trade amount is denominated by quantity or volume. Throws `BuildError` on a
[[nodiscard]] openpit::model::Order BuildOrder(const Row &row,
                                               openpit::param::AccountId acc);

// A final execution report carrying its pre-trade lock. The owned
// `model::ExecutionReport` holds every field except the lock; the owned
// `PreTradeLock` carries the single default-group entry at the lock price.
// `Raw` emits the C view with the lock pointer patched in. Move-only because
// the raw view borrows this object's storage and lock handle, so a `FillReport`
// must outlive every `Raw()` it hands to the engine. Mirrors the
// `buildFillReport`.
class FillReport {
public:
  FillReport(openpit::model::ExecutionReport report,
             openpit::pretrade::PreTradeLock lock)
      : m_report(std::move(report)),
        m_lock(std::make_unique<openpit::pretrade::PreTradeLock>(
            std::move(lock))) {}

  // The account this report addresses, for async per-account routing.
  [[nodiscard]] openpit::param::AccountId AccountId() const noexcept {
    return m_report.operation->accountId.value();
  }

  // Builds the native execution-report view with the fill lock attached. Valid
  // only while this `FillReport` is alive and unchanged.
  [[nodiscard]] OpenPitExecutionReport Raw() const noexcept {
    OpenPitExecutionReport raw = m_report.Raw();
    if (raw.fill.is_set) {
      raw.fill.value.lock = m_lock->Get();
    }
    return raw;
  }

private:
  openpit::model::ExecutionReport m_report;
  // Heap-held so the native lock handle's address is stable across moves.
  std::unique_ptr<openpit::pretrade::PreTradeLock> m_lock;
};

// Turns a FILL row into a final `FillReport`. The price column on a FILL is the
// lock price; when empty the most recent quote pushed for the instrument is
// reused. Throws `BuildError` on a malformed cell or a missing price. Mirrors
[[nodiscard]] FillReport BuildFillReport(const Row &row,
                                         openpit::param::AccountId acc,
                                         const MarketFeed &feed);

} // namespace spot_table
