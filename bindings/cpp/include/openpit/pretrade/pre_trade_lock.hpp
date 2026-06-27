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

#include "openpit/bytes.hpp"
#include "openpit/detail/handle.hpp"
#include "openpit/error.hpp"
#include "openpit/param.hpp"
#include "openpit/string.hpp"

#include <openpit.h>

#include <cstddef>
#include <cstdint>
#include <string>
#include <utility>
#include <vector>

// Pre-trade lock: the hot-path handle accumulating reserved-price records.
//
// `PreTradeLock` is a thin move-only RAII wrapper over
// `OpenPitPretradePreTradeLock*`. The mutating hot-path operations (`Push`,
// `PushMany`, `Len`, `IsEmpty`, `Merge`, `PricesView`) cross the C boundary
// directly and allocate nothing of their own; only the snapshot
// (`Entries`/`Prices`) and serialization helpers materialize owned data, and
// those are off the hot path. Expected validation failures surface as a thrown
// `openpit::Error`; construction failures throw as well. There is no reject
// channel here — a lock just stores prices.

namespace openpit::pretrade {

// One `(policy_group_id, price)` record stored in a lock. Mirrors the native
// runtime `OpenPitPretradePreTradeLockEntry`.
struct LockEntry {
  std::uint16_t policyGroupId = OPENPIT_DEFAULT_POLICY_GROUP_ID;
  ::openpit::param::Price price;

  LockEntry(std::uint16_t group, ::openpit::param::Price entryPrice)
      : policyGroupId(group), price(entryPrice) {}

  [[nodiscard]] static LockEntry FromRaw(
      const OpenPitPretradePreTradeLockEntry& raw) {
    return LockEntry(raw.policy_group_id,
                     ::openpit::param::Price::FromRaw(raw.price));
  }

  [[nodiscard]] OpenPitPretradePreTradeLockEntry Raw() const noexcept {
    OpenPitPretradePreTradeLockEntry raw{};
    raw.policy_group_id = policyGroupId;
    raw.price = price.Raw();
    return raw;
  }
};

namespace detail {

struct PreTradeLockDeleter {
  void operator()(OpenPitPretradePreTradeLock* handle) const noexcept {
    openpit_destroy_pretrade_pre_trade_lock(handle);
  }
};

struct PreTradeLockPricesDeleter {
  void operator()(OpenPitPretradePreTradeLockPrices* handle) const noexcept {
    openpit_destroy_pretrade_pre_trade_lock_prices(handle);
  }
};

struct PreTradeLockEntriesDeleter {
  void operator()(OpenPitPretradePreTradeLockEntries* handle) const noexcept {
    openpit_destroy_pretrade_pre_trade_lock_entries(handle);
  }
};

}  // namespace detail

// RAII pre-trade lock. Move-only; destruction releases the native handle.
class PreTradeLock {
 public:
  // Allocates an empty lock. The C constructor always succeeds.
  PreTradeLock() : m_handle(openpit_create_pretrade_pre_trade_lock()) {}

  // Adopts a caller-owned native lock handle (e.g. one returned by a
  // reservation or a deserializer).
  explicit PreTradeLock(OpenPitPretradePreTradeLock* handle) noexcept
      : m_handle(handle) {}

  [[nodiscard]] explicit operator bool() const noexcept {
    return static_cast<bool>(m_handle);
  }

  // Returns an independent deep copy of this lock.
  [[nodiscard]] PreTradeLock Clone() const {
    OpenPitPretradePreTradeLock* raw =
        openpit_pretrade_pre_trade_lock_clone(m_handle.Get());
    if (raw == nullptr) {
      throw Error("openpit_pretrade_pre_trade_lock_clone failed");
    }
    return PreTradeLock(raw);
  }

  // Total number of stored prices across all groups.
  [[nodiscard]] std::size_t Len() const noexcept {
    return openpit_pretrade_pre_trade_lock_len(m_handle.Get());
  }

  [[nodiscard]] bool IsEmpty() const noexcept {
    return openpit_pretrade_pre_trade_lock_is_empty(m_handle.Get());
  }

  // Appends `price` under `policyGroupId`. Throws `openpit::Error` when the
  // price fails domain validation.
  void Push(std::uint16_t policyGroupId, ::openpit::param::Price price) {
    OpenPitSharedString* error = nullptr;
    if (!openpit_pretrade_pre_trade_lock_push(m_handle.Get(), policyGroupId,
                                              price.Raw(), &error)) {
      ::openpit::detail::ThrowFromSharedString(
          error, "openpit_pretrade_pre_trade_lock_push failed");
    }
  }

  // Appends every record from `entries`, in order. On the first invalid price
  // nothing is appended and `openpit::Error` is thrown.
  void PushMany(const std::vector<LockEntry>& entries) {
    std::vector<OpenPitPretradePreTradeLockEntry> raw;
    raw.reserve(entries.size());
    for (const LockEntry& entry : entries) {
      raw.push_back(entry.Raw());
    }
    OpenPitSharedString* error = nullptr;
    if (!openpit_pretrade_pre_trade_lock_push_many(m_handle.Get(), raw.data(),
                                                   raw.size(), &error)) {
      ::openpit::detail::ThrowFromSharedString(
          error, "openpit_pretrade_pre_trade_lock_push_many failed");
    }
  }

  // Appends every record from `src` into this lock, leaving `src` unchanged.
  void Merge(const PreTradeLock& src) {
    OpenPitSharedString* error = nullptr;
    if (!openpit_pretrade_pre_trade_lock_merge(m_handle.Get(),
                                               src.m_handle.Get(), &error)) {
      ::openpit::detail::ThrowFromSharedString(
          error, "openpit_pretrade_pre_trade_lock_merge failed");
    }
  }

  // A snapshot of every `(policy_group_id, price)` record, in iteration order
  // (default-group records first, then each non-default group in insertion
  // order). Off the hot path: materializes owned values.
  [[nodiscard]] std::vector<LockEntry> Entries() const {
    ::openpit::detail::Handle<OpenPitPretradePreTradeLockEntries,
                              detail::PreTradeLockEntriesDeleter>
        entries(openpit_pretrade_pre_trade_lock_entries(m_handle.Get()));
    const OpenPitPretradePreTradeLockEntriesView view =
        openpit_pretrade_pre_trade_lock_entries_view(entries.Get());
    std::vector<LockEntry> out;
    if (view.ptr == nullptr || view.len == 0) {
      return out;
    }
    out.reserve(view.len);
    for (std::size_t index = 0; index < view.len; ++index) {
      out.push_back(LockEntry::FromRaw(view.ptr[index]));
    }
    return out;
  }

  // Every stored price, in the same iteration order as `Entries`. Off the hot
  // path.
  [[nodiscard]] std::vector<::openpit::param::Price> Prices() const {
    std::vector<LockEntry> entries = Entries();
    std::vector<::openpit::param::Price> out;
    out.reserve(entries.size());
    for (const LockEntry& entry : entries) {
      out.push_back(entry.price);
    }
    return out;
  }

  // Prices stored under `policyGroupId`, in insertion order. Throws
  // `openpit::Error` on a boundary failure.
  [[nodiscard]] std::vector<::openpit::param::Price> PricesOf(
      std::uint16_t policyGroupId) const {
    OpenPitParamPrice singlePrice{};
    OpenPitPretradePreTradeLockPrices* listRaw = nullptr;
    OpenPitSharedString* error = nullptr;
    const OpenPitPretradePreTradeLockPricesStatus status =
        openpit_pretrade_pre_trade_lock_prices_of(
            m_handle.Get(), policyGroupId, &singlePrice, &listRaw, &error);
    switch (status) {
      case OpenPitPretradePreTradeLockPricesStatus_Empty:
        return {};
      case OpenPitPretradePreTradeLockPricesStatus_One:
        return {::openpit::param::Price::FromRaw(singlePrice)};
      case OpenPitPretradePreTradeLockPricesStatus_List: {
        ::openpit::detail::Handle<OpenPitPretradePreTradeLockPrices,
                                  detail::PreTradeLockPricesDeleter>
            list(listRaw);
        return PricesFromView(
            openpit_pretrade_pre_trade_lock_prices_view(list.Get()));
      }
      default:
        ::openpit::detail::ThrowFromSharedString(
            error, "openpit_pretrade_pre_trade_lock_prices_of failed");
    }
  }

  //----------------------------------------------------------------------------
  // Serialization conveniences. Off the hot path. The msgpack/cbor/raw forms
  // produce bytes; json produces a UTF-8 string.

  [[nodiscard]] std::vector<std::uint8_t> ToMsgpack() const {
    return ToBytes(openpit_pretrade_pre_trade_lock_to_msgpack,
                   "openpit_pretrade_pre_trade_lock_to_msgpack failed");
  }

  [[nodiscard]] static PreTradeLock FromMsgpack(
      const std::vector<std::uint8_t>& payload) {
    return FromBytes(
        payload, openpit_create_pretrade_pre_trade_lock_from_msgpack,
        "openpit_create_pretrade_pre_trade_lock_from_msgpack failed");
  }

  [[nodiscard]] std::vector<std::uint8_t> ToCbor() const {
    return ToBytes(openpit_pretrade_pre_trade_lock_to_cbor,
                   "openpit_pretrade_pre_trade_lock_to_cbor failed");
  }

  [[nodiscard]] static PreTradeLock FromCbor(
      const std::vector<std::uint8_t>& payload) {
    return FromBytes(payload, openpit_create_pretrade_pre_trade_lock_from_cbor,
                     "openpit_create_pretrade_pre_trade_lock_from_cbor failed");
  }

  // The in-process binary-stable raw layout. Always succeeds.
  [[nodiscard]] std::vector<std::uint8_t> ToRaw() const {
    ::openpit::SharedBytes bytes(
        openpit_pretrade_pre_trade_lock_to_raw(m_handle.Get()));
    return bytes.ToVector();
  }

  [[nodiscard]] static PreTradeLock FromRaw(
      const std::vector<std::uint8_t>& payload) {
    return FromBytes(payload, openpit_create_pretrade_pre_trade_lock_from_raw,
                     "openpit_create_pretrade_pre_trade_lock_from_raw failed");
  }

  [[nodiscard]] std::string ToJson() const {
    OpenPitSharedString* error = nullptr;
    OpenPitSharedString* handle =
        openpit_pretrade_pre_trade_lock_to_json(m_handle.Get(), &error);
    if (handle == nullptr) {
      ::openpit::detail::ThrowFromSharedString(
          error, "openpit_pretrade_pre_trade_lock_to_json failed");
    }
    std::string result = ::openpit::SharedStringView(handle).ToString();
    openpit_destroy_shared_string(handle);
    return result;
  }

  [[nodiscard]] static PreTradeLock FromJson(std::string_view payload) {
    OpenPitSharedString* error = nullptr;
    OpenPitPretradePreTradeLock* raw =
        openpit_create_pretrade_pre_trade_lock_from_json(
            reinterpret_cast<const std::uint8_t*>(payload.data()),
            payload.size(), &error);
    if (raw == nullptr) {
      ::openpit::detail::ThrowFromSharedString(
          error, "openpit_create_pretrade_pre_trade_lock_from_json failed");
    }
    return PreTradeLock(raw);
  }

  // Borrows the native handle without transferring ownership.
  [[nodiscard]] OpenPitPretradePreTradeLock* Get() const noexcept {
    return m_handle.Get();
  }

  // Relinquishes ownership of the native handle to the caller.
  [[nodiscard]] OpenPitPretradePreTradeLock* Release() noexcept {
    return m_handle.Release();
  }

 private:
  [[nodiscard]] static std::vector<::openpit::param::Price> PricesFromView(
      const OpenPitPretradePreTradeLockPricesView& view) {
    std::vector<::openpit::param::Price> out;
    if (view.ptr == nullptr || view.len == 0) {
      return out;
    }
    out.reserve(view.len);
    for (std::size_t index = 0; index < view.len; ++index) {
      out.push_back(::openpit::param::Price::FromRaw(view.ptr[index]));
    }
    return out;
  }

  template <typename Encoder>
  [[nodiscard]] std::vector<std::uint8_t> ToBytes(Encoder encoder,
                                                  const char* fallback) const {
    OpenPitSharedString* error = nullptr;
    OpenPitSharedBytes* handle = encoder(m_handle.Get(), &error);
    if (handle == nullptr) {
      ::openpit::detail::ThrowFromSharedString(error, fallback);
    }
    ::openpit::SharedBytes bytes(handle);
    return bytes.ToVector();
  }

  template <typename Decoder>
  [[nodiscard]] static PreTradeLock FromBytes(
      const std::vector<std::uint8_t>& payload, Decoder decoder,
      const char* fallback) {
    OpenPitSharedString* error = nullptr;
    OpenPitPretradePreTradeLock* raw =
        decoder(payload.data(), payload.size(), &error);
    if (raw == nullptr) {
      ::openpit::detail::ThrowFromSharedString(error, fallback);
    }
    return PreTradeLock(raw);
  }

  ::openpit::detail::Handle<OpenPitPretradePreTradeLock,
                            detail::PreTradeLockDeleter>
      m_handle;
};

}  // namespace openpit::pretrade
