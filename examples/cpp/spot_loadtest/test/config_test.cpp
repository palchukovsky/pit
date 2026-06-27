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

// Config validation tests.
//
// Mirror of: examples/go/spot_loadtest/internal/config/config_phase2_test.go
//            examples/go/spot_loadtest/internal/config/helpers_phase2_test.go

#include "spot_loadtest/config/config.hpp"

#include <gtest/gtest.h>

#include <chrono>
#include <fstream>
#include <string>

namespace {

namespace config = spot_loadtest::config;

// A minimal but complete, valid config covering every section the generator
// consumes plus the required [run] block (mirror of the Go fullConfig const).
constexpr const char *kFullConfig = R"(
[run]
seed = 0xC0FFEE
total_ops = 1000
window = 100
window_unit = ops
observer = on

[arrival]
offered_rate = 50000
distribution = poisson

[reject]
target_rate = 0.05
tolerance = 0.005

[accounts]
count = 100

[concurrency]
active_accounts = 32

[async_engine]
strategy              = dynamic
max_queues            = 128
idle_cleanup          = 2s
sharded_workers       = 0
queue_capacity        = 0
slow_submit_threshold = 0

[instruments]
symbols = AAPL,SPX,MSFT
settlement = USD

[lifecycle]
p_open = 0.40
p_add = 0.15
p_partial_close = 0.25
p_full_close = 0.20

[funding]
trigger = balance_below
amount = 100000
seed = 1000000
top_up = 1000000

[cohort.chatty]
weight = 0.3
activity = 0.9
reject_propensity = 0.7
burst_len = 4
size_weights = 1:1,10:4,100:2
symbol_skew = zipf
zipf_s = 1.3

[cohort.steady]
weight = 0.7
activity = 0.5
reject_propensity = 0.25
burst_len = 2
size_weights = 1:2,10:3
symbol_skew = uniform
)";

// Replaces the first occurrence of `oldStr` in kFullConfig with `replacement`.
[[nodiscard]] std::string ReplaceLine(const std::string &oldStr,
                                      const std::string &replacement) {
  std::string s = kFullConfig;
  const std::size_t pos = s.find(oldStr);
  if (pos != std::string::npos) {
    s.replace(pos, oldStr.size(), replacement);
  }
  return s;
}

// Removes the first physical line containing `marker`.
[[nodiscard]] std::string RemoveLine(const std::string &marker) {
  std::string s = kFullConfig;
  const std::size_t pos = s.find(marker);
  if (pos == std::string::npos) {
    return s;
  }
  std::size_t lineStart = s.rfind('\n', pos);
  if (lineStart == std::string::npos) {
    lineStart = 0;
  }
  std::size_t lineEnd = s.find('\n', pos);
  if (lineEnd == std::string::npos) {
    lineEnd = s.size();
  }
  s.erase(lineStart, lineEnd - lineStart);
  return s;
}

// Removes every physical line containing any of the given markers (used to
// delete a whole section).
[[nodiscard]] std::string DropLines(std::string s,
                                    const std::vector<std::string> &markers) {
  std::string out;
  std::size_t i = 0;
  while (i < s.size()) {
    std::size_t end = s.find('\n', i);
    if (end == std::string::npos) {
      end = s.size();
    }
    const std::string line = s.substr(i, end - i);
    bool drop = false;
    for (const std::string &m : markers) {
      if (line.find(m) != std::string::npos) {
        drop = true;
        break;
      }
    }
    if (!drop) {
      out += line;
      if (end < s.size()) {
        out += '\n';
      }
    }
    i = end + 1;
  }
  return out;
}

[[nodiscard]] config::Config LoadString(const std::string &content) {
  return config::LoadFromString(content, "test.ini", "hash");
}

[[nodiscard]] bool Contains(const std::string &s, const std::string &sub) {
  return s.find(sub) != std::string::npos;
}

TEST(Config, LoadFullConfigValid) {
  const config::Config cfg = LoadString(kFullConfig);
  EXPECT_EQ(cfg.accounts.count, 100u);
  EXPECT_EQ(cfg.instruments.settlement, "USD");
  ASSERT_EQ(cfg.cohorts.size(), 2u);
  // Cohorts are sorted by name: chatty < steady.
  EXPECT_EQ(cfg.cohorts[0].name, "chatty");
  EXPECT_EQ(cfg.cohorts[1].name, "steady");
  EXPECT_EQ(cfg.cohorts[0].symbolSkew, config::SymbolSkew::Zipf);
  EXPECT_DOUBLE_EQ(cfg.cohorts[0].zipfS, 1.3);
  EXPECT_EQ(cfg.cohorts[0].sizeWeights.size(), 3u);
  EXPECT_EQ(cfg.funding.seed, cfg.funding.topUp);
  EXPECT_EQ(cfg.funding.seed.ToString(), "1000000");
  EXPECT_EQ(cfg.concurrency.activeAccounts, 32u);
  EXPECT_EQ(cfg.asyncEngine.strategy, config::AsyncEngineStrategy::Dynamic);
  EXPECT_EQ(cfg.asyncEngine.maxQueues, 128u);
  EXPECT_EQ(cfg.asyncEngine.idleCleanup, std::chrono::seconds(2));
  EXPECT_EQ(cfg.asyncEngine.shardedWorkers, 0);
  EXPECT_EQ(cfg.asyncEngine.queueCapacity, 0);
  EXPECT_EQ(cfg.asyncEngine.slowSubmitThreshold.count(), 0);
}

TEST(Config, StrictValidationRejectsBadValues) {
  struct Case {
    std::string name;
    std::string ini;
    std::string wantErr;
  };
  std::vector<Case> cases = {
      {"reject target_rate non-numeric",
       ReplaceLine("target_rate = 0.05", "target_rate = high"), "target_rate"},
      {"reject target_rate >= 1",
       ReplaceLine("target_rate = 0.05", "target_rate = 1.0"), "target_rate"},
      {"reject tolerance zero",
       ReplaceLine("tolerance = 0.005", "tolerance = 0"), "tolerance"},
      {"accounts count zero", ReplaceLine("count = 100", "count = 0"), "count"},
      {"concurrency active_accounts zero",
       ReplaceLine("active_accounts = 32", "active_accounts = 0"),
       "active_accounts"},
      {"concurrency active_accounts exceeds population",
       ReplaceLine("active_accounts = 32", "active_accounts = 101"),
       "exceeds accounts.count"},
      {"engine bad strategy",
       ReplaceLine("strategy              = dynamic",
                   "strategy              = turbocharged"),
       "strategy"},
      {"engine sharded without workers",
       ReplaceLine("strategy              = dynamic",
                   "strategy              = sharded"),
       "sharded_workers"},
      {"engine max_queues below active set",
       ReplaceLine("max_queues            = 128", "max_queues            = 16"),
       "max_queues"},
      {"engine max_queues non-numeric",
       ReplaceLine("max_queues            = 128",
                   "max_queues            = lots"),
       "max_queues"},
      {"engine idle_cleanup not a duration",
       ReplaceLine("idle_cleanup          = 2s",
                   "idle_cleanup          = soon"),
       "idle_cleanup"},
      {"engine idle_cleanup negative",
       ReplaceLine("idle_cleanup          = 2s", "idle_cleanup          = -1s"),
       "idle_cleanup"},
      {"engine queue_capacity negative",
       ReplaceLine("queue_capacity        = 0", "queue_capacity        = -1"),
       "queue_capacity"},
      {"engine slow_submit_threshold not a duration",
       ReplaceLine("slow_submit_threshold = 0", "slow_submit_threshold = soon"),
       "slow_submit_threshold"},
      {"engine slow_submit_threshold negative",
       ReplaceLine("slow_submit_threshold = 0", "slow_submit_threshold = -1s"),
       "slow_submit_threshold"},
      {"accounts count non-numeric", ReplaceLine("count = 100", "count = many"),
       "count"},
      {"instruments symbols blank entry",
       ReplaceLine("symbols = AAPL,SPX,MSFT", "symbols = AAPL,,MSFT"), "blank"},
      {"instruments duplicate symbol",
       ReplaceLine("symbols = AAPL,SPX,MSFT", "symbols = AAPL,AAPL"),
       "duplicate"},
      {"settlement collides with underlying",
       ReplaceLine("settlement = USD", "settlement = AAPL"),
       "must not also be an underlying"},
      {"lifecycle probability out of range",
       ReplaceLine("p_open = 0.40", "p_open = 1.5"), "p_open"},
      {"funding unknown trigger",
       ReplaceLine("trigger = balance_below", "trigger = on_tuesday"),
       "trigger"},
      {"funding amount non-positive",
       ReplaceLine("amount = 100000", "amount = 0"), "amount"},
      {"cohort weight non-positive", ReplaceLine("weight = 0.3", "weight = 0"),
       "weight"},
      {"cohort missing burst_len", RemoveLine("burst_len = 4"), "burst_len"},
      {"cohort bad size_weights",
       ReplaceLine("size_weights = 1:1,10:4,100:2", "size_weights = 1:1,bad"),
       "size_weights"},
      {"cohort zipf without zipf_s", RemoveLine("zipf_s = 1.3"), "zipf_s"},
      {"cohort zipf_s <= 1", ReplaceLine("zipf_s = 1.3", "zipf_s = 0.9"),
       "zipf_s"},
  };

  for (const Case &tc : cases) {
    try {
      (void)LoadString(tc.ini);
      ADD_FAILURE() << tc.name << ": expected error containing " << tc.wantErr;
    } catch (const config::ConfigError &e) {
      EXPECT_TRUE(Contains(e.what(), tc.wantErr))
          << tc.name << ": got " << e.what() << ", want " << tc.wantErr;
    }
  }
}

TEST(Config, NoCohortsIsError) {
  std::string ini = DropLines(
      kFullConfig,
      {"[cohort.chatty]", "[cohort.steady]", "weight = 0.3", "activity = 0.9",
       "reject_propensity = 0.7", "burst_len = 4",
       "size_weights = 1:1,10:4,100:2", "symbol_skew = zipf", "zipf_s = 1.3",
       "weight = 0.7", "activity = 0.5", "reject_propensity = 0.25",
       "burst_len = 2", "size_weights = 1:2,10:3", "symbol_skew = uniform"});
  try {
    (void)LoadString(ini);
    ADD_FAILURE() << "expected a missing-cohort error";
  } catch (const config::ConfigError &e) {
    EXPECT_TRUE(Contains(e.what(), "cohort"));
  }
}

TEST(Config, RunValidationNotRegressed) {
  EXPECT_THROW((void)LoadString(RemoveLine("window_unit = ops")),
               config::ConfigError);
  EXPECT_THROW(
      (void)LoadString(ReplaceLine("seed = 0xC0FFEE", "seed = notanumber")),
      config::ConfigError);
}

TEST(Config, EngineMaxQueuesUnlimited) {
  const config::Config cfg = LoadString(
      ReplaceLine("max_queues            = 128", "max_queues            = 0"));
  EXPECT_EQ(cfg.asyncEngine.maxQueues, 0u);
}

TEST(Config, EngineShardedStrategy) {
  std::string ini = ReplaceLine("strategy              = dynamic",
                                "strategy              = sharded");
  ini.replace(ini.find("sharded_workers       = 0"),
              std::string("sharded_workers       = 0").size(),
              "sharded_workers       = 4");
  const config::Config cfg = LoadString(ini);
  EXPECT_EQ(cfg.asyncEngine.strategy, config::AsyncEngineStrategy::Sharded);
  EXPECT_EQ(cfg.asyncEngine.shardedWorkers, 4);

  // sharded_workers = 0 with strategy = sharded is an error.
  EXPECT_THROW((void)LoadString(ReplaceLine("strategy              = dynamic",
                                            "strategy              = sharded")),
               config::ConfigError);
}

TEST(Config, EngineSharedKnobs) {
  std::string ini =
      ReplaceLine("queue_capacity        = 0", "queue_capacity        = 512");
  ini.replace(ini.find("slow_submit_threshold = 0"),
              std::string("slow_submit_threshold = 0").size(),
              "slow_submit_threshold = 250ms");
  const config::Config cfg = LoadString(ini);
  EXPECT_EQ(cfg.asyncEngine.queueCapacity, 512);
  EXPECT_EQ(cfg.asyncEngine.slowSubmitThreshold,
            std::chrono::milliseconds(250));
}

TEST(Config, ConcurrencyAndEngineSectionsRequired) {
  const std::string withoutConcurrency =
      DropLines(kFullConfig, {"[concurrency]", "active_accounts = 32"});
  EXPECT_THROW((void)LoadString(withoutConcurrency), config::ConfigError);

  const std::string withoutEngine = DropLines(
      kFullConfig, {"[async_engine]", "strategy              = dynamic",
                    "max_queues            = 128", "idle_cleanup          = 2s",
                    "sharded_workers       = 0", "queue_capacity        = 0",
                    "slow_submit_threshold = 0"});
  EXPECT_THROW((void)LoadString(withoutEngine), config::ConfigError);
}

// Loads the committed configs/baseline.ini through the strict parser, so the
// shipped reference config can never drift out of sync with the validation.
TEST(Config, BaselineConfigLoads) {
  const config::Config cfg = config::Load(SPOT_LOADTEST_BASELINE_INI);
  EXPECT_FALSE(cfg.cohorts.empty());
  EXPECT_FALSE(cfg.instruments.settlement.empty());
  EXPECT_FALSE(cfg.instruments.symbols.empty());
  EXPECT_EQ(cfg.run.seed, 0xC0FFEEu);
  EXPECT_EQ(cfg.run.totalOps, 2'000'000u);
  EXPECT_EQ(cfg.accounts.count, 10'000u);
  EXPECT_EQ(cfg.concurrency.activeAccounts, 1024u);
  EXPECT_EQ(cfg.asyncEngine.strategy, config::AsyncEngineStrategy::Dynamic);
  EXPECT_EQ(cfg.asyncEngine.maxQueues, 0u);
  EXPECT_EQ(cfg.asyncEngine.idleCleanup, std::chrono::seconds(5));
  EXPECT_EQ(cfg.cohorts.size(), 3u);
  EXPECT_EQ(cfg.instruments.symbols.size(), 10u);
  EXPECT_EQ(cfg.instruments.settlement, "USD");
  EXPECT_TRUE(cfg.run.observer);
  EXPECT_FALSE(cfg.hash.empty());
}

} // namespace
