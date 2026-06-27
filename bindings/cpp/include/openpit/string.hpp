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

#include "openpit/detail/handle.hpp"

#include <openpit.h>

#include <cstddef>
#include <string>
#include <string_view>

namespace openpit {

// Non-owning view over an `OpenPitStringView` (borrowed UTF-8 bytes).
//
// Lifetime mirrors the C contract: the view is valid only while the object that
// produced it is alive and unchanged. Copy out via `ToString()` to retain.
class StringView {
 public:
  StringView() noexcept = default;

  explicit StringView(OpenPitStringView view) noexcept : m_view(view) {}

  [[nodiscard]] const char* Data() const noexcept {
    return reinterpret_cast<const char*>(m_view.ptr);
  }

  [[nodiscard]] std::size_t Size() const noexcept { return m_view.len; }

  [[nodiscard]] bool Empty() const noexcept {
    return m_view.ptr == nullptr || m_view.len == 0;
  }

  // A borrowed `std::string_view`; empty when the source view is unset.
  [[nodiscard]] std::string_view View() const noexcept {
    if (m_view.ptr == nullptr) {
      return {};
    }
    return {Data(), m_view.len};
  }

  // Copies the bytes into an owning `std::string`.
  [[nodiscard]] std::string ToString() const { return std::string(View()); }

  [[nodiscard]] OpenPitStringView Raw() const noexcept { return m_view; }

 private:
  OpenPitStringView m_view{nullptr, 0};
};

// Builds an `OpenPitStringView` borrowing `value`'s bytes for the duration of a
// single C call. The caller must keep `value` alive across that call.
[[nodiscard]] inline OpenPitStringView MakeStringView(
    std::string_view value) noexcept {
  return OpenPitStringView{reinterpret_cast<const std::uint8_t*>(value.data()),
                           value.size()};
}

namespace detail {

struct SharedStringDeleter {
  void operator()(OpenPitSharedString* handle) const noexcept {
    openpit_destroy_shared_string(handle);
  }
};

}  // namespace detail

// Owning RAII wrapper over an `OpenPitSharedString` handle.
//
// Reading borrows bytes from the live handle (see `View()`); use `ToString()`
// to obtain an independent copy. Move-only.
class SharedString {
 public:
  SharedString() noexcept = default;

  explicit SharedString(OpenPitSharedString* handle) noexcept
      : m_handle(handle) {}

  [[nodiscard]] explicit operator bool() const noexcept {
    return static_cast<bool>(m_handle);
  }

  // Borrows the handle's bytes; valid only while this object is alive.
  [[nodiscard]] StringView View() const noexcept {
    return StringView(openpit_shared_string_view(m_handle.Get()));
  }

  [[nodiscard]] std::string ToString() const { return View().ToString(); }

  [[nodiscard]] OpenPitSharedString* Get() const noexcept {
    return m_handle.Get();
  }

 private:
  detail::Handle<OpenPitSharedString, detail::SharedStringDeleter> m_handle;
};

// Reads a caller-owned `OpenPitSharedString` view without taking ownership.
[[nodiscard]] inline StringView SharedStringView(
    const OpenPitSharedString* handle) noexcept {
  return StringView(openpit_shared_string_view(handle));
}

}  // namespace openpit
