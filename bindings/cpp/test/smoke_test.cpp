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

#include "openpit/openpit.hpp"

#include <gtest/gtest.h>

#include <string>

namespace {

TEST(Runtime, VersionIsNonEmpty) {
  const std::string version = openpit::GetVersion();
  EXPECT_FALSE(version.empty());
}

TEST(Runtime, BuildProfileReportsKnownKeys) {
  const std::string profile = openpit::GetBuildProfile();
  EXPECT_NE(profile.find("version="), std::string::npos);
  EXPECT_NE(profile.find("debug_assertions="), std::string::npos);
}

TEST(Param, PriceRoundTripsExactDecimalFromString) {
  const openpit::param::Price price =
      openpit::param::Price::FromString("185.25");
  EXPECT_EQ(price.ToString(), "185.25");
  EXPECT_FALSE(price.IsZero());
}

TEST(Param, PriceComparesExactly) {
  const openpit::param::Price low = openpit::param::Price::FromString("0.10");
  const openpit::param::Price high = openpit::param::Price::FromString("0.20");
  EXPECT_LT(low, high);
  EXPECT_NE(low, high);
  EXPECT_EQ(low.Compare(low), 0);
}

TEST(Param, QuantityZeroIsZero) {
  const openpit::param::Quantity zero = openpit::param::Quantity::FromInt64(0);
  EXPECT_TRUE(zero.IsZero());
}

TEST(Param, InvalidDecimalStringThrowsError) {
  EXPECT_THROW(
      { (void)openpit::param::Price::FromString("not-a-number"); },
      openpit::Error);
}

TEST(Engine, BuilderConstructsForEverySyncPolicy) {
  EXPECT_NO_THROW(
      { openpit::EngineBuilder builder(openpit::SyncPolicy::None); });
  EXPECT_NO_THROW(
      { openpit::EngineBuilder builder(openpit::SyncPolicy::Full); });
  EXPECT_NO_THROW(
      { openpit::EngineBuilder builder(openpit::SyncPolicy::Account); });
}

TEST(Engine, BuildWithoutPoliciesThrows) {
  openpit::EngineBuilder builder(openpit::SyncPolicy::Full);
  // The slice does not register policies yet; the boundary failure surfaces as
  // a thrown openpit::Error. The builder handle is released by RAII regardless.
  EXPECT_THROW({ openpit::Engine engine = builder.Build(); }, openpit::Error);
}

TEST(Reject, CarriesScopeAndCode) {
  const openpit::reject::Reject reject(
      "order_size_limit", openpit::reject::RejectScope::Order,
      openpit::reject::RejectCode::OrderQtyExceedsLimit, "qty too large",
      "max 100");
  EXPECT_EQ(reject.scope, openpit::reject::RejectScope::Order);
  EXPECT_EQ(reject.code, openpit::reject::RejectCode::OrderQtyExceedsLimit);
  EXPECT_EQ(reject.policy, "order_size_limit");
}

TEST(Reject, RawRoundTripPreservesScopeAndCode) {
  openpit::reject::Reject reject(
      "pnl_kill_switch", openpit::reject::RejectScope::Account,
      openpit::reject::RejectCode::PnlKillSwitchTriggered, "loss breached",
      "threshold");
  reject.userData = 42;

  const OpenPitPretradeReject raw = reject.Raw();
  const openpit::reject::Reject restored =
      openpit::reject::Reject::FromRaw(raw);

  EXPECT_EQ(restored.scope, openpit::reject::RejectScope::Account);
  EXPECT_EQ(restored.code, openpit::reject::RejectCode::PnlKillSwitchTriggered);
  EXPECT_EQ(restored.policy, "pnl_kill_switch");
  EXPECT_EQ(restored.reason, "loss breached");
  EXPECT_EQ(restored.details, "threshold");
  EXPECT_EQ(restored.userData, 42u);
}

}  // namespace
