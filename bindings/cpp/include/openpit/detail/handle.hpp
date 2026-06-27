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

#include <utility>

namespace openpit::detail {

// Move-only RAII owner for an opaque `OpenPit*` C handle.
//
// `CType` is the opaque struct (e.g. `OpenPitEngine`). `Deleter` is a stateless
// type with `static void operator()(CType*)` (or a callable instance) that
// forwards to the matching `openpit_destroy_*` / `openpit_*_free` C function.
// The deleter contract of every such C function accepts null, so the moved-from
// and default states (null) are always safe to destroy.
//
// The wrapper is intentionally thin: it stores one pointer and never copies.
template <typename CType, typename Deleter>
class Handle {
 public:
  Handle() noexcept = default;

  explicit Handle(CType* raw) noexcept : m_raw(raw) {}

  Handle(const Handle&) = delete;
  Handle& operator=(const Handle&) = delete;

  Handle(Handle&& other) noexcept
      : m_raw(std::exchange(other.m_raw, nullptr)) {}

  Handle& operator=(Handle&& other) noexcept {
    if (this != &other) {
      Reset(std::exchange(other.m_raw, nullptr));
    }
    return *this;
  }

  ~Handle() { Reset(nullptr); }

  // Returns the borrowed raw pointer without transferring ownership.
  [[nodiscard]] CType* Get() const noexcept { return m_raw; }

  [[nodiscard]] explicit operator bool() const noexcept {
    return m_raw != nullptr;
  }

  // Relinquishes ownership and returns the raw pointer; the caller becomes
  // responsible for destruction.
  [[nodiscard]] CType* Release() noexcept {
    return std::exchange(m_raw, nullptr);
  }

  // Destroys the current handle (if any) and adopts `raw`.
  void Reset(CType* raw) noexcept {
    CType* old = std::exchange(m_raw, raw);
    if (old != nullptr) {
      Deleter{}(old);
    }
  }

 private:
  CType* m_raw = nullptr;
};

}  // namespace openpit::detail
