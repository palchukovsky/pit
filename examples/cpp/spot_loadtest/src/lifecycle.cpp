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

#include "spot_loadtest/generator/lifecycle.hpp"

#include "spot_loadtest/generator/rng.hpp"

#include <cstdint>
#include <string>
#include <vector>

namespace spot_loadtest::generator {

std::uint64_t Lifecycle::StateQty(const std::string &account,
                                  const std::string &underlying) const {
  auto it = m_positions.find(PosKey{account, underlying});
  return it == m_positions.end() ? 0 : it->second;
}

Action Lifecycle::Decide(Rng &g, const std::string &account,
                         const std::string &underlying) {
  const std::uint64_t qty = StateQty(account, underlying);

  if (qty == 0) {
    // Flat: open with p_open, else idle.
    if (g.Bernoulli(m_cfg.pOpen)) {
      return Action::Open;
    }
    return Action::Idle;
  }

  // Long: weighted choice among add / partial / full / idle.
  std::vector<double> weights;
  weights.push_back(m_cfg.pAdd);
  weights.push_back(m_cfg.pPartialClose);
  weights.push_back(m_cfg.pFullClose);
  std::vector<Action> acts = {Action::Add, Action::PartialClose,
                              Action::FullClose};
  double residual = 1.0 - (weights[0] + weights[1] + weights[2]);
  if (residual < 0) {
    residual = 0;
  }
  weights.push_back(residual);
  acts.push_back(Action::Idle);

  const int idx = g.PickWeighted(Cumulative(weights));
  return acts[static_cast<std::size_t>(idx)];
}

void Lifecycle::ApplyOpenOrAdd(const std::string &account,
                               const std::string &underlying,
                               std::uint64_t lots) {
  m_positions[PosKey{account, underlying}] += lots;
}

void Lifecycle::ApplyClose(const std::string &account,
                           const std::string &underlying, std::uint64_t lots) {
  const PosKey key{account, underlying};
  const std::uint64_t qty = StateQty(account, underlying);
  if (lots >= qty) {
    m_positions.erase(key);
    return;
  }
  m_positions[key] = qty - lots;
}

std::uint64_t Lifecycle::CloseLots(Rng &g, const std::string &account,
                                   const std::string &underlying, bool full) {
  const std::uint64_t qty = StateQty(account, underlying);
  if (qty == 0) {
    return 0;
  }
  if (full || qty == 1) {
    return qty;
  }
  // Partial: sell between 1 and qty-1 lots inclusive.
  return static_cast<std::uint64_t>(g.IntN(static_cast<int>(qty - 1))) + 1;
}

} // namespace spot_loadtest::generator
