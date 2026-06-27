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

#include "openpit/error.hpp"

#include <exception>
#include <utility>

namespace openpit::detail {

inline thread_local std::exception_ptr g_pendingCallbackException;

inline void ClearPendingCallbackException() noexcept {
  g_pendingCallbackException = nullptr;
}

inline void CaptureCurrentCallbackException() noexcept {
  g_pendingCallbackException = std::current_exception();
}

[[nodiscard]] inline bool HasPendingCallbackException() noexcept {
  return g_pendingCallbackException != nullptr;
}

inline void ThrowIfPendingCallbackException(const char* fallback) {
  // `fallback` documents the caller's context only when no exception is
  // pending; on the reachable paths below `pending` is always non-null, so
  // every `catch` arm rethrows and the function never falls through.
  static_cast<void>(fallback);
  if (g_pendingCallbackException == nullptr) {
    return;
  }
  std::exception_ptr pending =
      std::exchange(g_pendingCallbackException, std::exception_ptr{});
  try {
    std::rethrow_exception(pending);
  } catch (const ::openpit::Error& error) {
    throw ::openpit::Error(error.Message());
  } catch (const std::exception& error) {
    throw ::openpit::Error(error.what());
  } catch (...) {
    throw ::openpit::Error("unknown callback error");
  }
}

}  // namespace openpit::detail
