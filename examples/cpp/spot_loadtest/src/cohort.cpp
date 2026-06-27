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

#include "spot_loadtest/generator/cohort.hpp"

#include "spot_loadtest/config/config.hpp"
#include "spot_loadtest/generator/rng.hpp"

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <functional>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

namespace spot_loadtest::generator {
namespace {

[[nodiscard]] double Sum(const std::vector<double> &xs) {
  double s = 0.0;
  for (const double x : xs) {
    s += x;
  }
  return s;
}

// Builds the cumulative symbol-selection table for a cohort. Uniform: weight 1
// each; Zipf: the i-th symbol (1-based) has weight 1/i^s.
[[nodiscard]] std::vector<double>
SymbolWeights(const config::Cohort &cc,
              const std::vector<std::string> &symbols) {
  std::vector<double> w(symbols.size());
  switch (cc.symbolSkew) {
  case config::SymbolSkew::Uniform:
    for (double &x : w) {
      x = 1.0;
    }
    break;
  case config::SymbolSkew::Zipf:
    for (std::size_t i = 0; i < w.size(); ++i) {
      w[i] = 1.0 / std::pow(static_cast<double>(i + 1), cc.zipfS);
    }
    break;
  }
  return Cumulative(w);
}

// Deals count accounts to cohorts proportional to weights using the
// largest-remainder method, guaranteeing the counts sum to exactly count.
[[nodiscard]] std::vector<Account>
AssignAccounts(std::uint64_t count, const std::vector<double> &weights) {
  if (count == 0) {
    throw std::runtime_error("population: account count must be > 0");
  }
  const double total = Sum(weights);
  if (total <= 0) {
    throw std::runtime_error("population: total cohort weight must be > 0");
  }

  const int n = static_cast<int>(count);
  std::vector<double> quotas(weights.size());
  std::vector<int> counts(weights.size(), 0);
  int assigned = 0;
  for (std::size_t i = 0; i < weights.size(); ++i) {
    const double q = static_cast<double>(n) * weights[i] / total;
    quotas[i] = q;
    counts[i] = static_cast<int>(std::floor(q));
    assigned += counts[i];
  }
  for (; assigned < n; ++assigned) {
    int best = -1;
    double bestResidual = -1e300;
    for (std::size_t i = 0; i < weights.size(); ++i) {
      const double residual = quotas[i] - static_cast<double>(counts[i]);
      if (residual > bestResidual) {
        bestResidual = residual;
        best = static_cast<int>(i);
      }
    }
    counts[static_cast<std::size_t>(best)]++;
  }

  std::vector<Account> accounts;
  accounts.reserve(static_cast<std::size_t>(n));
  int idx = 0;
  for (std::size_t ci = 0; ci < counts.size(); ++ci) {
    for (int k = 0; k < counts[ci]; ++k) {
      char buf[32];
      std::snprintf(buf, sizeof(buf), "acct-%06d", idx);
      accounts.push_back(Account{std::string(buf), static_cast<int>(ci)});
      ++idx;
    }
  }
  return accounts;
}

} // namespace

std::unique_ptr<Population> Population::Build(const config::Config &cfg) {
  const std::vector<std::string> &symbols = cfg.instruments.symbols;
  if (symbols.empty()) {
    throw std::runtime_error("population: no instruments");
  }

  std::vector<Cohort> cohorts(cfg.cohorts.size());
  std::vector<double> weights(cfg.cohorts.size());
  std::vector<double> rejectWeights(cfg.cohorts.size());
  for (std::size_t i = 0; i < cfg.cohorts.size(); ++i) {
    const config::Cohort &cc = cfg.cohorts[i];
    std::vector<double> sizeWeights(cc.sizeWeights.size());
    for (std::size_t j = 0; j < cc.sizeWeights.size(); ++j) {
      sizeWeights[j] = cc.sizeWeights[j].weight;
    }
    cohorts[i].cfg = cc;
    cohorts[i].sizeCum = Cumulative(sizeWeights);
    cohorts[i].symbolCum = SymbolWeights(cc, symbols);
    weights[i] = cc.weight;
    rejectWeights[i] = cc.rejectPropensity;
  }

  std::vector<Account> accounts = AssignAccounts(cfg.accounts.count, weights);

  // Fall back to uniform reject propensity when every cohort is zero.
  if (Sum(rejectWeights) == 0) {
    for (double &w : rejectWeights) {
      w = 1.0;
    }
  }

  std::vector<std::vector<int>> byCohort(cohorts.size());
  for (std::size_t i = 0; i < accounts.size(); ++i) {
    byCohort[static_cast<std::size_t>(accounts[i].cohort)].push_back(
        static_cast<int>(i));
  }
  std::vector<double> admitWeights(cohorts.size());
  for (std::size_t i = 0; i < cfg.cohorts.size(); ++i) {
    if (byCohort[i].empty()) {
      admitWeights[i] = 0.0;
      continue;
    }
    admitWeights[i] = cfg.cohorts[i].weight * cfg.cohorts[i].activity;
  }
  if (Sum(admitWeights) == 0) {
    for (std::size_t i = 0; i < admitWeights.size(); ++i) {
      if (!byCohort[i].empty()) {
        admitWeights[i] = weights[i];
      }
    }
  }

  auto pop = std::make_unique<Population>();
  pop->m_cohorts = std::move(cohorts);
  pop->m_cohortCum = Cumulative(weights);
  pop->m_accounts = std::move(accounts);
  pop->m_symbols = symbols;
  pop->m_rejectCum = Cumulative(rejectWeights);
  pop->m_byCohort = std::move(byCohort);
  pop->m_admitCum = Cumulative(admitWeights);
  return pop;
}

int Population::AdmitAccount(Rng &g,
                             const std::function<bool(int)> &active) const {
  const int cohortIdx = g.PickWeighted(m_admitCum);
  const std::vector<int> &list =
      m_byCohort[static_cast<std::size_t>(cohortIdx)];
  if (list.empty()) {
    return ScanInactive(active);
  }
  const int start = g.IntN(static_cast<int>(list.size()));
  for (std::size_t off = 0; off < list.size(); ++off) {
    const int idx = list[(static_cast<std::size_t>(start) + off) % list.size()];
    if (!active(idx)) {
      return idx;
    }
  }
  return ScanInactive(active);
}

int Population::ScanInactive(const std::function<bool(int)> &active) const {
  for (std::size_t i = 0; i < m_accounts.size(); ++i) {
    if (!active(static_cast<int>(i))) {
      return static_cast<int>(i);
    }
  }
  return -1;
}

int Population::PickSymbol(Rng &g, int cohortIdx) const {
  return g.PickWeighted(
      m_cohorts[static_cast<std::size_t>(cohortIdx)].symbolCum);
}

std::uint64_t Population::PickSize(Rng &g, int cohortIdx) const {
  const int idx =
      g.PickWeighted(m_cohorts[static_cast<std::size_t>(cohortIdx)].sizeCum);
  return m_cohorts[static_cast<std::size_t>(cohortIdx)]
      .cfg.sizeWeights[static_cast<std::size_t>(idx)]
      .quantity;
}

} // namespace spot_loadtest::generator
