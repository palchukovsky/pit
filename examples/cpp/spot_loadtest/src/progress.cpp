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

#include "spot_loadtest/progress/progress.hpp"

#include <chrono>
#include <cstdint>
#include <cstdio>
#include <string>

namespace spot_loadtest::progress {
namespace {

constexpr double kScaleKilo = 1'000.0;
constexpr double kScaleMega = 1'000'000.0;
constexpr double kPctScale = 100.0;

// CR + spaces (wider than a typical terminal) + CR, leaving no visible noise.
constexpr const char *kClearLine =
    "\r                                                                "
    "              \r";

[[nodiscard]] double Seconds(std::chrono::nanoseconds d) {
  return std::chrono::duration<double>(d).count();
}

// Renders a duration compactly (e.g. "1h23m", "4m05s", "12s").
[[nodiscard]] std::string FmtDuration(std::chrono::nanoseconds d) {
  auto secs = std::chrono::duration_cast<std::chrono::seconds>(d);
  if (secs.count() < 0) {
    secs = std::chrono::seconds(0);
  }
  const long long h = secs.count() / 3600;
  const long long m = (secs.count() % 3600) / 60;
  const long long s = secs.count() % 60;
  char buf[32];
  if (h > 0) {
    std::snprintf(buf, sizeof(buf), "%lldh%02lldm", h, m);
  } else if (m > 0) {
    std::snprintf(buf, sizeof(buf), "%lldm%02llds", m, s);
  } else {
    std::snprintf(buf, sizeof(buf), "%llds", s);
  }
  return buf;
}

// Renders a large integer compactly (e.g. "1.2M", "45K", "999").
[[nodiscard]] std::string FmtCount(std::uint64_t n) {
  char buf[32];
  if (static_cast<double>(n) >= kScaleMega) {
    std::snprintf(buf, sizeof(buf), "%.1fM",
                  static_cast<double>(n) / kScaleMega);
  } else if (static_cast<double>(n) >= kScaleKilo) {
    std::snprintf(buf, sizeof(buf), "%.1fK",
                  static_cast<double>(n) / kScaleKilo);
  } else {
    std::snprintf(buf, sizeof(buf), "%llu", static_cast<unsigned long long>(n));
  }
  return buf;
}

// Renders an ops/s rate compactly (e.g. "50.0K/s", "1.2M/s").
[[nodiscard]] std::string FmtRate(double r) {
  char buf[32];
  if (r >= kScaleMega) {
    std::snprintf(buf, sizeof(buf), "%.1fM/s", r / kScaleMega);
  } else if (r >= kScaleKilo) {
    std::snprintf(buf, sizeof(buf), "%.1fK/s", r / kScaleKilo);
  } else {
    std::snprintf(buf, sizeof(buf), "%.0f/s", r);
  }
  return buf;
}

} // namespace

std::string FormatLine(const measurement::LiveCounters &c, std::uint64_t total,
                       std::chrono::nanoseconds elapsed) {
  const std::string elapsedStr = FmtDuration(elapsed);
  const double elapsedSec = Seconds(elapsed);

  std::string remainStr = "?";
  if (total > 0 && c.decided > 0 && c.decided <= total) {
    const double fraction =
        static_cast<double>(c.decided) / static_cast<double>(total);
    if (fraction > 0) {
      const double totalSec = elapsedSec / fraction;
      const double remainSec = totalSec - elapsedSec;
      if (remainSec > 0) {
        remainStr = FmtDuration(std::chrono::nanoseconds(
            static_cast<std::int64_t>(remainSec * 1e9)));
      } else {
        remainStr = "0s";
      }
    }
  }

  std::string rateStr = "?";
  if (elapsedSec > 0 && c.decided > 0) {
    rateStr = FmtRate(static_cast<double>(c.decided) / elapsedSec);
  }

  std::string pctStr = "?%";
  if (total > 0) {
    char buf[16];
    std::snprintf(buf, sizeof(buf), "%.1f%%",
                  static_cast<double>(c.decided) * kPctScale /
                      static_cast<double>(total));
    pctStr = buf;
  }

  char line[256];
  std::snprintf(
      line, sizeof(line),
      "elapsed %s | remaining ~%s | %s decided | in-flight %lld | rate %s | %s",
      elapsedStr.c_str(), remainStr.c_str(), FmtCount(c.decided).c_str(),
      static_cast<long long>(c.inFlight), rateStr.c_str(), pctStr.c_str());
  return line;
}

Reporter::Reporter(std::ostream &out, const driver::LiveSource &source,
                   std::uint64_t total, std::chrono::milliseconds interval)
    : m_out(out), m_source(source), m_total(total), m_interval(interval) {
  if (m_interval.count() <= 0) {
    m_interval = kDefaultInterval;
  }
}

Reporter::~Reporter() {
  if (m_started) {
    Stop();
  }
}

void Reporter::Start(std::chrono::steady_clock::time_point start) {
  m_started = true;
  m_thread = std::thread([this, start] { Loop(start); });
}

void Reporter::Stop() {
  {
    std::lock_guard<std::mutex> lock(m_mutex);
    if (m_done) {
      return;
    }
    m_done = true;
  }
  m_cv.notify_all();
  if (m_thread.joinable()) {
    m_thread.join();
  }
  m_out << kClearLine << std::flush;
  m_started = false;
}

void Reporter::Loop(std::chrono::steady_clock::time_point start) {
  std::unique_lock<std::mutex> lock(m_mutex);
  while (!m_done) {
    if (m_cv.wait_for(lock, m_interval, [this] { return m_done; })) {
      return;
    }
    lock.unlock();
    Render(std::chrono::steady_clock::now(), start);
    lock.lock();
  }
}

void Reporter::Render(std::chrono::steady_clock::time_point now,
                      std::chrono::steady_clock::time_point start) {
  const measurement::LiveCounters c = m_source.Counters();
  const auto elapsed =
      std::chrono::duration_cast<std::chrono::nanoseconds>(now - start);
  m_out << '\r' << FormatLine(c, m_total, elapsed) << std::flush;
}

} // namespace spot_loadtest::progress
