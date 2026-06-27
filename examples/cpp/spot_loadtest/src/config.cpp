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

#include "spot_loadtest/config/config.hpp"

#include "spot_loadtest/decimal.hpp"
#include "spot_loadtest/sha256.hpp"

#include <algorithm>
#include <cctype>
#include <cstdint>
#include <fstream>
#include <map>
#include <optional>
#include <sstream>
#include <string>
#include <vector>

namespace spot_loadtest::config {
namespace {

// A parsed INI: ordered sections, each an ordered list of key=value pairs.
struct Section {
  std::string name;
  std::map<std::string, std::string> keys;
};

struct IniFile {
  std::vector<Section> sections;

  [[nodiscard]] const Section *Find(const std::string &name) const {
    for (const Section &s : sections) {
      if (s.name == name) {
        return &s;
      }
    }
    return nullptr;
  }
};

[[nodiscard]] std::string Trim(const std::string &s) {
  std::size_t a = 0;
  std::size_t b = s.size();
  while (a < b && std::isspace(static_cast<unsigned char>(s[a]))) {
    ++a;
  }
  while (b > a && std::isspace(static_cast<unsigned char>(s[b - 1]))) {
    --b;
  }
  return s.substr(a, b - a);
}

// Strips an inline ';' comment from a value (gopkg.in/ini.v1 semantics: ';' and
// '#' start a comment). The baseline config relies on inline ';' comments.
[[nodiscard]] std::string StripInlineComment(const std::string &s) {
  for (std::size_t i = 0; i < s.size(); ++i) {
    if (s[i] == ';' || s[i] == '#') {
      return s.substr(0, i);
    }
  }
  return s;
}

[[nodiscard]] IniFile ParseIni(const std::string &content) {
  IniFile file;
  Section current;
  current.name = ""; // the unnamed default section.
  bool haveCurrent = false;
  std::istringstream in(content);
  std::string line;
  while (std::getline(in, line)) {
    std::string trimmed = Trim(line);
    if (trimmed.empty() || trimmed[0] == ';' || trimmed[0] == '#') {
      continue;
    }
    if (trimmed.front() == '[' && trimmed.back() == ']') {
      if (haveCurrent) {
        file.sections.push_back(std::move(current));
      }
      current = Section{};
      current.name = Trim(trimmed.substr(1, trimmed.size() - 2));
      haveCurrent = true;
      continue;
    }
    const std::size_t eq = trimmed.find('=');
    if (eq == std::string::npos) {
      continue;
    }
    std::string key = Trim(trimmed.substr(0, eq));
    std::string value = Trim(StripInlineComment(trimmed.substr(eq + 1)));
    if (!haveCurrent) {
      haveCurrent = true;
    }
    current.keys[key] = value;
  }
  if (haveCurrent) {
    file.sections.push_back(std::move(current));
  }
  return file;
}

// Parses an unsigned integer the way gopkg.in/ini.v1 Uint64 does: a leading
// "0x"/"0X" is hex, "0o" octal, "0b" binary, otherwise base 10. Returns nullopt
// on any malformed input.
[[nodiscard]] std::optional<std::uint64_t> ParseUint(const std::string &raw) {
  std::string s = Trim(raw);
  if (s.empty()) {
    return std::nullopt;
  }
  int base = 10;
  std::size_t i = 0;
  if (s.size() > 2 && s[0] == '0') {
    const char c =
        static_cast<char>(std::tolower(static_cast<unsigned char>(s[1])));
    if (c == 'x') {
      base = 16;
      i = 2;
    } else if (c == 'o') {
      base = 8;
      i = 2;
    } else if (c == 'b') {
      base = 2;
      i = 2;
    }
  }
  if (i >= s.size()) {
    return std::nullopt;
  }
  std::uint64_t value = 0;
  for (; i < s.size(); ++i) {
    const char c =
        static_cast<char>(std::tolower(static_cast<unsigned char>(s[i])));
    int digit = 0;
    if (c >= '0' && c <= '9') {
      digit = c - '0';
    } else if (c >= 'a' && c <= 'f') {
      digit = 10 + (c - 'a');
    } else {
      return std::nullopt;
    }
    if (digit >= base) {
      return std::nullopt;
    }
    value = value * static_cast<std::uint64_t>(base) +
            static_cast<std::uint64_t>(digit);
  }
  return value;
}

[[nodiscard]] std::optional<std::int64_t> ParseInt(const std::string &raw) {
  std::string s = Trim(raw);
  if (s.empty()) {
    return std::nullopt;
  }
  bool negative = false;
  std::size_t i = 0;
  if (s[0] == '+' || s[0] == '-') {
    negative = s[0] == '-';
    i = 1;
  }
  if (i >= s.size()) {
    return std::nullopt;
  }
  std::int64_t value = 0;
  for (; i < s.size(); ++i) {
    if (s[i] < '0' || s[i] > '9') {
      return std::nullopt;
    }
    value = value * 10 + (s[i] - '0');
  }
  return negative ? -value : value;
}

[[nodiscard]] std::optional<double> ParseDouble(const std::string &raw) {
  std::string s = Trim(raw);
  if (s.empty()) {
    return std::nullopt;
  }
  try {
    std::size_t pos = 0;
    const double v = std::stod(s, &pos);
    if (pos != s.size()) {
      return std::nullopt;
    }
    return v;
  } catch (...) {
    return std::nullopt;
  }
}

// Parses a Go-style duration string (e.g. "5s", "2ms", "1m", "250ms", "-1s",
// "0s"). Supports ns/us/µs/ms/s/m/h units and a leading sign. Returns nullopt
// on malformed input. Matches time.ParseDuration closely enough for the
// harness.
[[nodiscard]] std::optional<std::chrono::nanoseconds>
ParseDuration(const std::string &raw) {
  std::string s = Trim(raw);
  if (s.empty()) {
    return std::nullopt;
  }
  bool negative = false;
  std::size_t i = 0;
  if (s[0] == '+' || s[0] == '-') {
    negative = s[0] == '-';
    i = 1;
  }
  if (i >= s.size()) {
    return std::nullopt;
  }
  if (s.substr(i) == "0") {
    return std::chrono::nanoseconds(0);
  }
  long double totalNs = 0.0L;
  bool sawComponent = false;
  while (i < s.size()) {
    // Number (integer or decimal).
    const std::size_t numStart = i;
    while (i < s.size() && ((s[i] >= '0' && s[i] <= '9') || s[i] == '.')) {
      ++i;
    }
    if (i == numStart) {
      return std::nullopt;
    }
    const std::string numStr = s.substr(numStart, i - numStart);
    double num = 0.0;
    try {
      std::size_t pos = 0;
      num = std::stod(numStr, &pos);
      if (pos != numStr.size()) {
        return std::nullopt;
      }
    } catch (...) {
      return std::nullopt;
    }
    // Unit.
    const std::size_t unitStart = i;
    // Multi-byte 'µ' (U+00B5, bytes 0xC2 0xB5) for microseconds.
    std::string unit;
    if (i + 1 < s.size() && static_cast<unsigned char>(s[i]) == 0xC2 &&
        static_cast<unsigned char>(s[i + 1]) == 0xB5) {
      unit = "us";
      i += 2;
      if (i < s.size() && s[i] == 's') {
        ++i; // "µs"
      }
    } else {
      while (i < s.size() && !(s[i] >= '0' && s[i] <= '9') && s[i] != '.') {
        unit.push_back(s[i]);
        ++i;
      }
    }
    if (i == unitStart && unit.empty()) {
      return std::nullopt;
    }
    double unitNs = 0.0;
    if (unit == "ns") {
      unitNs = 1.0;
    } else if (unit == "us" || unit == "µs") {
      unitNs = 1e3;
    } else if (unit == "ms") {
      unitNs = 1e6;
    } else if (unit == "s") {
      unitNs = 1e9;
    } else if (unit == "m") {
      unitNs = 60.0 * 1e9;
    } else if (unit == "h") {
      unitNs = 3600.0 * 1e9;
    } else {
      return std::nullopt;
    }
    totalNs += static_cast<long double>(num) * unitNs;
    sawComponent = true;
  }
  if (!sawComponent) {
    return std::nullopt;
  }
  if (negative) {
    totalNs = -totalNs;
  }
  return std::chrono::nanoseconds(static_cast<std::int64_t>(totalNs));
}

[[nodiscard]] std::optional<Decimal>
ParsePositiveDecimal(const std::string &raw) {
  try {
    const Decimal d = Decimal::FromString(Trim(raw));
    if (!d.IsPositive()) {
      return std::nullopt;
    }
    return d;
  } catch (...) {
    return std::nullopt;
  }
}

// Reads a required key from a section, throwing on absence.
[[nodiscard]] const std::string &RequireKey(const Section &sec,
                                            const std::string &sectionLabel,
                                            const std::string &key) {
  auto it = sec.keys.find(key);
  if (it == sec.keys.end()) {
    throw ConfigError(sectionLabel + ": key " + key + " is required");
  }
  return it->second;
}

[[nodiscard]] const Section &RequireSection(const IniFile &file,
                                            const std::string &path,
                                            const std::string &name) {
  const Section *sec = file.Find(name);
  if (sec == nullptr) {
    throw ConfigError("config " + path + ": section [" + name +
                      "] is required");
  }
  return *sec;
}

[[nodiscard]] double RequireFloat(const Section &sec, const std::string &label,
                                  const std::string &key) {
  const std::string &raw = RequireKey(sec, label, key);
  std::optional<double> v = ParseDouble(raw);
  if (!v) {
    throw ConfigError(label + ": " + key + ": must be a number, got \"" + raw +
                      "\"");
  }
  return *v;
}

[[nodiscard]] double RequireUnitFloat(const Section &sec,
                                      const std::string &label,
                                      const std::string &key) {
  const double v = RequireFloat(sec, label, key);
  if (v < 0 || v > 1) {
    throw ConfigError(label + ": " + key + ": must be in [0, 1]");
  }
  return v;
}

[[nodiscard]] double RequirePositiveFloat(const Section &sec,
                                          const std::string &label,
                                          const std::string &key) {
  const double v = RequireFloat(sec, label, key);
  if (v <= 0) {
    throw ConfigError(label + ": " + key + ": must be > 0");
  }
  return v;
}

void LoadRun(const IniFile &file, const std::string &path, Run &r) {
  const Section &sec = RequireSection(file, path, "run");
  const std::string label = "config " + path + " [run]";

  std::optional<std::uint64_t> seed = ParseUint(RequireKey(sec, label, "seed"));
  if (!seed) {
    throw ConfigError(label + ": seed: must be a non-negative integer");
  }
  r.seed = *seed;

  const bool hasTotalOps = sec.keys.count("total_ops") != 0;
  const bool hasDuration = sec.keys.count("duration") != 0;
  if (hasTotalOps && hasDuration) {
    throw ConfigError(label +
                      ": total_ops and duration are mutually exclusive; "
                      "provide exactly one");
  }
  if (hasTotalOps) {
    std::optional<std::uint64_t> n = ParseUint(sec.keys.at("total_ops"));
    if (!n || *n == 0) {
      throw ConfigError(label + ": total_ops: must be a positive integer");
    }
    r.totalOps = *n;
  } else if (hasDuration) {
    const std::string d = Trim(sec.keys.at("duration"));
    if (d.empty()) {
      throw ConfigError(label + ": duration: must be a non-empty duration");
    }
    r.duration = d;
  } else {
    throw ConfigError(label +
                      ": exactly one of total_ops or duration is required");
  }

  std::optional<std::uint64_t> window =
      ParseUint(RequireKey(sec, label, "window"));
  if (!window || *window == 0) {
    throw ConfigError(label + ": window: must be a positive integer");
  }
  r.window = *window;

  const std::string wu = Trim(RequireKey(sec, label, "window_unit"));
  if (wu == "ops") {
    r.windowUnit = WindowUnit::Ops;
  } else if (wu == "wall") {
    r.windowUnit = WindowUnit::Wall;
  } else {
    throw ConfigError(label + ": window_unit: must be ops or wall");
  }

  const std::string obs = Trim(RequireKey(sec, label, "observer"));
  if (obs == "on") {
    r.observer = true;
  } else if (obs == "off") {
    r.observer = false;
  } else {
    throw ConfigError(label + ": observer: must be on or off");
  }
}

void LoadArrival(const IniFile &file, Arrival &a) {
  const Section *sec = file.Find("arrival");
  if (sec == nullptr) {
    return;
  }
  auto it = sec->keys.find("offered_rate");
  if (it != sec->keys.end()) {
    if (std::optional<std::uint64_t> n = ParseUint(it->second)) {
      a.offeredRate = *n;
    }
  }
}

void LoadReportDelay(const IniFile &file, ReportDelay &d) {
  const Section *sec = file.Find("report_delay");
  if (sec == nullptr) {
    return;
  }
  if (auto it = sec->keys.find("distribution"); it != sec->keys.end()) {
    const std::string v = Trim(it->second);
    if (v == "lognormal") {
      d.distribution = ReportDelayDistribution::Lognormal;
    } else if (v == "fixed") {
      d.distribution = ReportDelayDistribution::Fixed;
    }
  }
  if (auto it = sec->keys.find("mean"); it != sec->keys.end()) {
    d.mean = Trim(it->second);
  }
  if (auto it = sec->keys.find("sigma"); it != sec->keys.end()) {
    if (std::optional<double> v = ParseDouble(it->second)) {
      d.sigma = *v;
    }
  }
}

void LoadReject(const IniFile &file, const std::string &path, Reject &r) {
  const Section &sec = RequireSection(file, path, "reject");
  const std::string label = "config " + path + " [reject]";
  const double rate = RequireUnitFloat(sec, label, "target_rate");
  if (rate >= 1) {
    throw ConfigError(label + ": target_rate: must be < 1");
  }
  r.targetRate = rate;
  const double tol = RequireFloat(sec, label, "tolerance");
  if (tol <= 0 || tol > 1) {
    throw ConfigError(label + ": tolerance: must be in (0, 1]");
  }
  r.tolerance = tol;
}

void LoadAccounts(const IniFile &file, const std::string &path, Accounts &a) {
  const Section &sec = RequireSection(file, path, "accounts");
  const std::string label = "config " + path + " [accounts]";
  std::optional<std::uint64_t> n = ParseUint(RequireKey(sec, label, "count"));
  if (!n || *n == 0) {
    throw ConfigError(label + ": count: must be a positive integer");
  }
  a.count = *n;
}

void LoadConcurrency(const IniFile &file, const std::string &path,
                     Concurrency &c, std::uint64_t population) {
  const Section &sec = RequireSection(file, path, "concurrency");
  const std::string label = "config " + path + " [concurrency]";
  std::optional<std::uint64_t> n =
      ParseUint(RequireKey(sec, label, "active_accounts"));
  if (!n || *n == 0) {
    throw ConfigError(label + ": active_accounts: must be a positive integer");
  }
  if (*n > population) {
    throw ConfigError(label + ": active_accounts: " + std::to_string(*n) +
                      " exceeds accounts.count " + std::to_string(population) +
                      " (active set cannot exceed the population)");
  }
  c.activeAccounts = *n;
}

void LoadAsyncEngine(const IniFile &file, const std::string &path,
                     AsyncEngine &e, std::uint64_t activeAccounts) {
  const Section &sec = RequireSection(file, path, "async_engine");
  const std::string label = "config " + path + " [async_engine]";

  const std::string strat = Trim(RequireKey(sec, label, "strategy"));
  if (strat == "dynamic") {
    e.strategy = AsyncEngineStrategy::Dynamic;
  } else if (strat == "sharded") {
    e.strategy = AsyncEngineStrategy::Sharded;
  } else {
    throw ConfigError(label + ": strategy: must be \"dynamic\" or \"sharded\"");
  }

  std::optional<std::uint64_t> mq =
      ParseUint(RequireKey(sec, label, "max_queues"));
  if (!mq) {
    throw ConfigError(label + ": max_queues: must be a non-negative integer "
                              "(0 = unlimited)");
  }
  if (e.strategy == AsyncEngineStrategy::Dynamic) {
    if (*mq != 0 && *mq < activeAccounts) {
      throw ConfigError(
          label + ": max_queues: " + std::to_string(*mq) +
          " must be 0 (unlimited) or >= concurrency.active_accounts " +
          std::to_string(activeAccounts) + " (dynamic strategy)");
    }
  }
  e.maxQueues = *mq;

  const std::string &icRaw = RequireKey(sec, label, "idle_cleanup");
  std::optional<std::chrono::nanoseconds> idle = ParseDuration(icRaw);
  if (!idle) {
    throw ConfigError(label + ": idle_cleanup: must be a duration (e.g. 5s)");
  }
  if (idle->count() < 0) {
    throw ConfigError(label + ": idle_cleanup: must be >= 0 (0 = disabled)");
  }
  e.idleCleanup = *idle;

  std::optional<std::int64_t> sw =
      ParseInt(RequireKey(sec, label, "sharded_workers"));
  if (!sw) {
    throw ConfigError(label +
                      ": sharded_workers: must be a non-negative integer");
  }
  if (e.strategy == AsyncEngineStrategy::Sharded && *sw <= 0) {
    throw ConfigError(label +
                      ": sharded_workers: must be > 0 when strategy = sharded");
  }
  e.shardedWorkers = static_cast<int>(*sw);

  std::optional<std::int64_t> qc =
      ParseInt(RequireKey(sec, label, "queue_capacity"));
  if (!qc) {
    throw ConfigError(label +
                      ": queue_capacity: must be a non-negative integer "
                      "(0 = engine default 1024)");
  }
  if (*qc < 0) {
    throw ConfigError(label + ": queue_capacity: must be >= 0");
  }
  e.queueCapacity = static_cast<int>(*qc);

  std::string sstRaw = Trim(RequireKey(sec, label, "slow_submit_threshold"));
  if (sstRaw == "0") {
    sstRaw = "0s";
  }
  std::optional<std::chrono::nanoseconds> sst = ParseDuration(sstRaw);
  if (!sst) {
    throw ConfigError(label +
                      ": slow_submit_threshold: must be a duration (e.g. 1m) "
                      "or 0 (engine default)");
  }
  if (sst->count() < 0) {
    throw ConfigError(label + ": slow_submit_threshold: must be >= 0 "
                              "(0 = engine default 1m)");
  }
  e.slowSubmitThreshold = *sst;
}

void LoadInstruments(const IniFile &file, const std::string &path,
                     Instruments &inst) {
  const Section &sec = RequireSection(file, path, "instruments");
  const std::string label = "config " + path + " [instruments]";
  const std::string &raw = RequireKey(sec, label, "symbols");

  std::vector<std::string> symbols;
  std::map<std::string, bool> seen;
  std::stringstream ss(raw);
  std::string part;
  int idx = 0;
  while (std::getline(ss, part, ',')) {
    ++idx;
    const std::string s = Trim(part);
    if (s.empty()) {
      throw ConfigError(label + ": symbols: entry " + std::to_string(idx) +
                        " is blank");
    }
    if (seen.count(s) != 0) {
      throw ConfigError(label + ": symbols: duplicate entry \"" + s + "\"");
    }
    seen[s] = true;
    symbols.push_back(s);
  }
  if (symbols.empty()) {
    throw ConfigError(label + ": symbols: must list at least one instrument");
  }
  inst.symbols = std::move(symbols);

  std::string settlement = "USD";
  if (auto it = sec.keys.find("settlement"); it != sec.keys.end()) {
    settlement = Trim(it->second);
    if (settlement.empty()) {
      throw ConfigError(label + ": settlement: must be a non-empty asset code");
    }
  }
  if (seen.count(settlement) != 0) {
    throw ConfigError(label + ": settlement \"" + settlement +
                      "\" must not also be an underlying symbol");
  }
  inst.settlement = settlement;
}

void LoadLifecycle(const IniFile &file, const std::string &path,
                   Lifecycle &lc) {
  const Section &sec = RequireSection(file, path, "lifecycle");
  const std::string label = "config " + path + " [lifecycle]";
  lc.pOpen = RequireUnitFloat(sec, label, "p_open");
  lc.pAdd = RequireUnitFloat(sec, label, "p_add");
  lc.pPartialClose = RequireUnitFloat(sec, label, "p_partial_close");
  lc.pFullClose = RequireUnitFloat(sec, label, "p_full_close");
}

void LoadFunding(const IniFile &file, const std::string &path, Funding &fd) {
  const Section &sec = RequireSection(file, path, "funding");
  const std::string label = "config " + path + " [funding]";
  const std::string trig = Trim(RequireKey(sec, label, "trigger"));
  if (trig != "balance_below") {
    throw ConfigError(label + ": trigger: must be \"balance_below\"");
  }
  fd.trigger = FundingTrigger::BalanceBelow;

  const std::string &amountRaw = RequireKey(sec, label, "amount");
  std::optional<Decimal> threshold = ParsePositiveDecimal(amountRaw);
  if (!threshold) {
    throw ConfigError(label + ": amount: must be a decimal > 0");
  }
  fd.threshold = *threshold;

  fd.seed = *threshold;
  if (auto it = sec.keys.find("seed"); it != sec.keys.end()) {
    std::optional<Decimal> v = ParsePositiveDecimal(it->second);
    if (!v) {
      throw ConfigError(label + ": seed: must be a decimal > 0");
    }
    fd.seed = *v;
  }
  fd.topUp = *threshold;
  if (auto it = sec.keys.find("top_up"); it != sec.keys.end()) {
    std::optional<Decimal> v = ParsePositiveDecimal(it->second);
    if (!v) {
      throw ConfigError(label + ": top_up: must be a decimal > 0");
    }
    fd.topUp = *v;
  }
}

[[nodiscard]] std::vector<SizeBucket>
ParseSizeWeights(const Section &sec, const std::string &label) {
  const std::string &raw = RequireKey(sec, label, "size_weights");
  std::vector<SizeBucket> buckets;
  std::stringstream ss(raw);
  std::string part;
  int idx = 0;
  while (std::getline(ss, part, ',')) {
    ++idx;
    const std::string p = Trim(part);
    if (p.empty()) {
      throw ConfigError(label + ": size_weights: entry " + std::to_string(idx) +
                        " is blank");
    }
    const std::size_t colon = p.find(':');
    if (colon == std::string::npos) {
      throw ConfigError(label + ": size_weights: entry \"" + p +
                        "\" must be qty:weight");
    }
    std::optional<std::uint64_t> qty = ParseUint(Trim(p.substr(0, colon)));
    if (!qty || *qty == 0) {
      throw ConfigError(label + ": size_weights: quantity in \"" + p +
                        "\" must be a positive integer");
    }
    std::optional<double> w = ParseDouble(Trim(p.substr(colon + 1)));
    if (!w || *w <= 0) {
      throw ConfigError(label + ": size_weights: weight in \"" + p +
                        "\" must be a positive number");
    }
    buckets.push_back(SizeBucket{*qty, *w});
  }
  if (buckets.empty()) {
    throw ConfigError(label +
                      ": size_weights: must list at least one qty:weight "
                      "bucket");
  }
  return buckets;
}

[[nodiscard]] Cohort ParseCohort(const Section &sec, const std::string &name) {
  const std::string label = "[cohort." + name + "]";
  Cohort c;
  c.name = name;
  c.weight = RequirePositiveFloat(sec, label, "weight");
  c.activity = RequireUnitFloat(sec, label, "activity");
  c.rejectPropensity = RequireUnitFloat(sec, label, "reject_propensity");

  std::optional<std::uint64_t> burst =
      ParseUint(RequireKey(sec, label, "burst_len"));
  if (!burst || *burst == 0) {
    throw ConfigError(label + ": burst_len: must be a positive integer");
  }
  c.burstLen = *burst;

  c.sizeWeights = ParseSizeWeights(sec, label);

  const std::string skew = Trim(RequireKey(sec, label, "symbol_skew"));
  if (skew == "uniform") {
    c.symbolSkew = SymbolSkew::Uniform;
  } else if (skew == "zipf") {
    c.symbolSkew = SymbolSkew::Zipf;
    const double s = RequireFloat(sec, label, "zipf_s");
    if (s <= 1) {
      throw ConfigError(label + ": zipf_s: must be > 1");
    }
    c.zipfS = s;
  } else {
    throw ConfigError(label + ": symbol_skew: must be \"uniform\" or \"zipf\"");
  }
  return c;
}

void LoadCohorts(const IniFile &file, const std::string &path, Config &cfg) {
  std::vector<Cohort> cohorts;
  for (const Section &sec : file.sections) {
    const std::string prefix = "cohort.";
    if (sec.name.rfind(prefix, 0) != 0) {
      continue;
    }
    const std::string name = Trim(sec.name.substr(prefix.size()));
    if (name.empty()) {
      throw ConfigError("config " + path +
                        ": [cohort.]: cohort name must not be empty");
    }
    cohorts.push_back(ParseCohort(sec, name));
  }
  if (cohorts.empty()) {
    throw ConfigError("config " + path +
                      ": at least one [cohort.<name>] section is required");
  }
  std::sort(cohorts.begin(), cohorts.end(),
            [](const Cohort &a, const Cohort &b) { return a.name < b.name; });
  cfg.cohorts = std::move(cohorts);
}

} // namespace

std::string ToString(WindowUnit unit) {
  return unit == WindowUnit::Ops ? "ops" : "wall";
}

std::string ToString(AsyncEngineStrategy strategy) {
  return strategy == AsyncEngineStrategy::Dynamic ? "dynamic" : "sharded";
}

Config LoadFromString(const std::string &content, const std::string &path,
                      const std::string &hash) {
  const IniFile file = ParseIni(content);

  Config cfg;
  cfg.path = path;
  cfg.hash = hash;

  LoadRun(file, path, cfg.run);
  LoadArrival(file, cfg.arrival);
  LoadReportDelay(file, cfg.reportDelay);
  LoadReject(file, path, cfg.reject);
  LoadAccounts(file, path, cfg.accounts);
  LoadConcurrency(file, path, cfg.concurrency, cfg.accounts.count);
  LoadAsyncEngine(file, path, cfg.asyncEngine, cfg.concurrency.activeAccounts);
  LoadInstruments(file, path, cfg.instruments);
  LoadLifecycle(file, path, cfg.lifecycle);
  LoadFunding(file, path, cfg.funding);
  LoadCohorts(file, path, cfg);

  return cfg;
}

Config Load(const std::string &path) {
  std::ifstream in(path, std::ios::binary);
  if (!in) {
    throw ConfigError("config: read \"" + path + "\": cannot open file");
  }
  std::ostringstream buffer;
  buffer << in.rdbuf();
  const std::string content = buffer.str();
  const std::string hash = Sha256Hex(content);
  return LoadFromString(content, path, hash);
}

} // namespace spot_loadtest::config
