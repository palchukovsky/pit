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

// Driver integration tests (require the native core).
//
// Mirror of: examples/go/spot_loadtest/internal/driver/driver_test.go
//            examples/go/spot_loadtest/internal/driver/doc_backing_test.go
//
// Run with the native runtime resolvable at load time, e.g.:
//   OPENPIT_RUNTIME_LIBRARY=$(pwd)/target/release/libopenpit_ffi.dylib

#include "spot_loadtest/config/config.hpp"
#include "spot_loadtest/decimal.hpp"
#include "spot_loadtest/driver/driver.hpp"
#include "spot_loadtest/env/env.hpp"
#include "spot_loadtest/generator/generator.hpp"
#include "spot_loadtest/reporter/reporter.hpp"

#include <gtest/gtest.h>

#include <cstdint>
#include <memory>
#include <sstream>
#include <string>

namespace {

namespace config = spot_loadtest::config;
namespace driver = spot_loadtest::driver;
namespace gen = spot_loadtest::generator;
using spot_loadtest::Decimal;

[[nodiscard]] Decimal Dec(const std::string &s) {
  return Decimal::FromString(s);
}

[[nodiscard]] config::Config TestConfig(std::uint64_t seed,
                                        std::uint64_t totalOps, double target) {
  config::Config c;
  c.run.seed = seed;
  c.run.totalOps = totalOps;
  c.run.window = 1000;
  c.run.windowUnit = config::WindowUnit::Ops;
  c.run.observer = true;
  c.arrival.offeredRate = 0; // unpaced (saturated).
  c.reject = config::Reject{target, 0.01};
  c.accounts = config::Accounts{200};
  c.concurrency = config::Concurrency{64};
  c.asyncEngine.strategy = config::AsyncEngineStrategy::Dynamic;
  c.asyncEngine.maxQueues = 256;
  c.asyncEngine.idleCleanup = std::chrono::seconds(2);
  c.instruments.symbols = {"AAPL", "SPX",  "MSFT", "AMZN", "GOOG",
                           "META", "TSLA", "NVDA", "JPM",  "BAC"};
  c.instruments.settlement = "USD";
  c.lifecycle = config::Lifecycle{0.40, 0.15, 0.25, 0.20};
  c.funding.trigger = config::FundingTrigger::BalanceBelow;
  c.funding.threshold = Dec("100000");
  c.funding.seed = Dec("1000000");
  c.funding.topUp = Dec("1000000");
  c.cohorts = {
      config::Cohort{"chatty",
                     0.2,
                     0.9,
                     0.7,
                     4,
                     {{1, 1}, {10, 4}, {100, 2}},
                     config::SymbolSkew::Zipf,
                     1.3},
      config::Cohort{"steady",
                     0.5,
                     0.5,
                     0.25,
                     2,
                     {{1, 2}, {10, 3}, {100, 1}},
                     config::SymbolSkew::Uniform,
                     0.0},
      config::Cohort{"dormant",
                     0.3,
                     0.1,
                     0.05,
                     1,
                     {{1, 5}, {10, 1}},
                     config::SymbolSkew::Uniform,
                     0.0},
  };
  return c;
}

// The Phase-3 / Phase-4 integration gate: a moderate stream through the REAL
// asyncengine, asserting the per-op oracle agrees, submission is open-loop, the
// windows are populated, inner metrics fire, the overhead probe ran, and the
// checksum changed.
TEST(Driver, OraclePipeline) {
  const config::Config cfg = TestConfig(0xC0FFEE, 30000, 0.05);
  const std::unique_ptr<gen::Stream> stream = gen::Generate(cfg);
  ASSERT_GT(stream->stats.accepts, 0u);
  ASSERT_GT(stream->stats.rejects, 0u);

  driver::Config dcfg;
  dcfg.observer = true;
  dcfg.collectors = 16;
  dcfg.windowSize = 1000;
  dcfg.windowUnit = spot_loadtest::measurement::WindowUnit::Ops;
  dcfg.overheadProbes = 50;

  const driver::RunResult result = driver::Run(*stream, dcfg);
  const driver::Stats &stats = result.stats;
  const auto &snap = result.snapshot;

  const std::uint64_t wantOrderEvents =
      stream->stats.orderChecks +
      (stream->stats.fundings - stream->stats.seeds);
  EXPECT_EQ(stats.orderChecks, wantOrderEvents);
  EXPECT_EQ(stats.settlements, stream->stats.settlements);
  EXPECT_GT(stats.sampleCount, 0);

  // Open-loop witness: peak in-flight well above the active set.
  EXPECT_GT(stats.maxInFlight,
            static_cast<std::int64_t>(cfg.concurrency.activeAccounts));

  EXPECT_FALSE(snap.windows.empty());
  EXPECT_GT(snap.orderCheck.count, 0);
  EXPECT_GT(snap.orderCheck.p99.count(), 0);
  EXPECT_GT(snap.serviceTime.count, 0);
  EXPECT_GT(snap.innerMetrics.dequeues, 0);
  EXPECT_GT(snap.innerMetrics.completes, 0);
  EXPECT_GT(snap.overhead.probes, 0);
  EXPECT_GT(snap.overhead.distribution.p50.count(), 0);
  EXPECT_NE(stats.checksum, 0u);
}

// Raises the reject target so a large fraction rejects, exercising the
// reject-code mapping heavily; the oracle must still agree.
TEST(Driver, OpenLoopHighRejectRate) {
  const config::Config cfg = TestConfig(0xBEEF, 20000, 0.20);
  const std::unique_ptr<gen::Stream> stream = gen::Generate(cfg);
  ASSERT_GT(stream->stats.rejects, 0u);

  driver::Config dcfg;
  dcfg.observer = false;
  dcfg.collectors = 16;
  dcfg.windowSize = 1000;
  dcfg.overheadProbes = 0;

  const driver::RunResult result = driver::Run(*stream, dcfg);
  EXPECT_GT(result.stats.rejects, 0u);
  EXPECT_GT(result.stats.maxInFlight,
            static_cast<std::int64_t>(cfg.concurrency.activeAccounts));
  EXPECT_EQ(result.snapshot.innerMetrics.dequeues, 0);
}

// Drives the full bounded-concurrency path via FromAppConfig and asserts the
// oracle agrees, submission stays open-loop, and a healthy run reports ZERO
// backpressure.
TEST(Driver, BoundedConcurrency) {
  const config::Config cfg = TestConfig(0xC0FFEE, 30000, 0.05);
  const std::unique_ptr<gen::Stream> stream = gen::Generate(cfg);
  ASSERT_EQ(stream->stats.seeds, cfg.accounts.count);

  driver::Config dcfg = driver::FromAppConfig(cfg);
  dcfg.collectors = 16;
  dcfg.overheadProbes = 0;

  const driver::RunResult result = driver::Run(*stream, dcfg);
  EXPECT_EQ(result.stats.backpressure, 0u);
  EXPECT_NE(result.stats.checksum, 0u);
  EXPECT_GT(result.stats.maxInFlight,
            static_cast<std::int64_t>(cfg.concurrency.activeAccounts));
  const std::uint64_t wantOrderEvents =
      stream->stats.orderChecks +
      (stream->stats.fundings - stream->stats.seeds);
  EXPECT_EQ(result.stats.orderChecks, wantOrderEvents);
}

// Exercises the PACED offered-rate path; submission still overlaps decisions.
TEST(Driver, PacedSubmission) {
  config::Config cfg = TestConfig(0x5EED, 8000, 0.05);
  cfg.arrival.offeredRate = 100000;
  const std::unique_ptr<gen::Stream> stream = gen::Generate(cfg);

  driver::Config dcfg;
  dcfg.observer = false;
  dcfg.collectors = 16;
  dcfg.windowSize = 1000;
  dcfg.overheadProbes = 0;

  const driver::RunResult result = driver::Run(*stream, dcfg);
  EXPECT_GT(result.stats.sampleCount, 0);
  EXPECT_GE(result.stats.maxInFlight, 2);
  EXPECT_GT(result.snapshot.orderCheck.count, 0);
}

// Exercises the sharded dispatch path end-to-end.
TEST(Driver, ShardedStrategy) {
  config::Config cfg = TestConfig(0x5ADED, 15000, 0.05);
  cfg.asyncEngine.strategy = config::AsyncEngineStrategy::Sharded;
  cfg.asyncEngine.shardedWorkers = 3;
  const std::unique_ptr<gen::Stream> stream = gen::Generate(cfg);
  ASSERT_GT(stream->stats.accepts, 0u);
  ASSERT_GT(stream->stats.rejects, 0u);

  driver::Config dcfg = driver::FromAppConfig(cfg);
  dcfg.collectors = 16;
  dcfg.overheadProbes = 0;

  const driver::RunResult result = driver::Run(*stream, dcfg);
  const std::uint64_t wantOrderEvents =
      stream->stats.orderChecks +
      (stream->stats.fundings - stream->stats.seeds);
  EXPECT_EQ(result.stats.orderChecks, wantOrderEvents);
  EXPECT_EQ(result.stats.settlements, stream->stats.settlements);
  EXPECT_EQ(result.stats.backpressure, 0u);
  EXPECT_GE(result.stats.maxInFlight, 2);
  EXPECT_FALSE(result.snapshot.windows.empty());
  EXPECT_GT(result.snapshot.orderCheck.count, 0);
}

// Source: examples/cpp/spot_loadtest/README.md - Build and run
//
// The doc-backing test: loads the committed baseline.ini, runs a reduced
// end-to-end through the REAL engine, and renders the report asserting every
// named block is present, the run is oracle-clean, backpressure is zero, and
// the anti-DCE checksum is non-zero.
TEST(Driver, DocBackingBaselineRecipe) {
  config::Config baseCfg = config::Load(SPOT_LOADTEST_BASELINE_INI);
  ASSERT_FALSE(baseCfg.cohorts.empty());
  ASSERT_FALSE(baseCfg.instruments.symbols.empty());

  // Reduced run: same seed + cohort structure as the baseline, smaller scale.
  config::Config reduced = baseCfg;
  reduced.run.totalOps = 30000;
  reduced.run.window = 5000;
  reduced.accounts.count = 500;
  reduced.concurrency.activeAccounts = 64;
  reduced.asyncEngine.maxQueues = 0;
  reduced.asyncEngine.idleCleanup = std::chrono::seconds(2);

  const std::unique_ptr<gen::Stream> stream = gen::Generate(reduced);
  ASSERT_GT(stream->stats.orderChecks, 0u);

  driver::Config dcfg = driver::FromAppConfig(reduced);
  dcfg.collectors = 16;
  dcfg.overheadProbes = 50;

  const driver::RunResult result = driver::Run(*stream, dcfg);
  EXPECT_GT(result.stats.orderChecks, 0u);
  EXPECT_GT(result.stats.accepts, 0u);
  EXPECT_GT(result.stats.rejects, 0u);
  EXPECT_EQ(result.snapshot.backpressure, 0u);
  EXPECT_NE(result.snapshot.checksum, 0u);
  EXPECT_GE(result.snapshot.maxInFlight, 2);
  EXPECT_FALSE(result.snapshot.windows.empty());
  EXPECT_GT(result.snapshot.orderCheck.count, 0);

  // Render and assert all named blocks (use a synthetic env so the test is
  // hermetic).
  spot_loadtest::env::Env e;
  e.host.cpuModel = "doc-backing-test (synthetic)";
  e.core.version = "0.0.0";
  e.core.profile = "release";
  std::ostringstream buf;
  spot_loadtest::reporter::Write(buf, e, reduced, "configs/baseline.ini",
                                 result.snapshot, stream->stats);
  const std::string out = buf.str();
  for (const char *block :
       {"=== Headline:", "=== Environment ===", "=== Workload ===",
        "=== Trajectory", "=== Distribution", "=== Diagnostics",
        "=== Disclaimer ==="}) {
    EXPECT_NE(out.find(block), std::string::npos) << "missing block " << block;
  }
  EXPECT_NE(out.find("0 (healthy"), std::string::npos);
  EXPECT_NE(out.find("Open-Loop Order-Check Latency"), std::string::npos);
  EXPECT_NE(out.find("DIAGNOSTIC, NOT the headline"), std::string::npos);
  EXPECT_NE(out.find("What IS measured"), std::string::npos);
  EXPECT_NE(out.find("What is NOT measured"), std::string::npos);
  EXPECT_NE(out.find("configs/baseline.ini"), std::string::npos);
  EXPECT_NE(out.find("Anti-DCE checksum"), std::string::npos);
}

} // namespace
