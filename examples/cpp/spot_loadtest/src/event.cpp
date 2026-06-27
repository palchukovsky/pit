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

#include "spot_loadtest/generator/event.hpp"

#include <sstream>
#include <string>

namespace spot_loadtest::generator {

std::string ToString(Side side) {
  switch (side) {
  case Side::Buy:
    return "BUY";
  case Side::Sell:
    return "SELL";
  }
  return "UNKNOWN";
}

std::string ToString(RejectReason reason) {
  switch (reason) {
  case RejectReason::None:
    return "";
  case RejectReason::InsufficientFunds:
    return "InsufficientFunds";
  case RejectReason::AccountAssetNotConfigured:
    return "SpotAccountAssetNotConfigured";
  }
  return "";
}

std::string ToString(FundingKind kind) {
  return kind == FundingKind::Absolute ? "Absolute" : "Delta";
}

std::string ToString(EventKind kind) {
  switch (kind) {
  case EventKind::OrderCheck:
    return "ORDERCHECK";
  case EventKind::Settlement:
    return "SETTLEMENT";
  case EventKind::Funding:
    return "FUNDING";
  }
  return "UNKNOWN";
}

std::string Stream::Serialize() const {
  std::ostringstream out;
  for (const Event &e : events) {
    out << e.seq << '|' << ToString(e.kind) << "|vt0=" << e.virtualT0.count()
        << "ns|" << e.account << '|' << e.underlying << '|' << e.settlement
        << '|' << ToString(e.side) << "|q=" << e.quantity.ToString()
        << "|p=" << e.price.ToString() << "|corr=" << e.correlationId
        << "|acc=" << (e.accept ? "true" : "false")
        << "|rej=" << ToString(e.reason) << "|fk=" << ToString(e.fundingKind)
        << "|fa=" << e.fundingAsset << "|fv=" << e.fundingAmount.ToString()
        << "|seed=" << (e.fundingIsSeed ? "true" : "false") << '|';
    for (const Balance &b : e.post) {
      out << '{' << b.asset << ":av=" << b.available.ToString()
          << ":hd=" << b.held.ToString() << '}';
    }
    out << '\n';
  }
  return out.str();
}

} // namespace spot_loadtest::generator
