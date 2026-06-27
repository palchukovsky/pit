#!/usr/bin/env bash
# Copyright The Pit Project Owners. All rights reserved.
# SPDX-License-Identifier: Apache-2.0
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
# Please see https://openpit.dev and the OWNERS file for details.

set -euo pipefail

: "${OPENPIT_VERSION:?OPENPIT_VERSION is required}"

asset="openpit-cpp--${OPENPIT_VERSION}.tar.gz"
base_url="https://github.com/openpitkit/pit/releases/download/v${OPENPIT_VERSION}"
work_root="/tmp/openpit-cpp-release-e2e"
install_dir="${work_root}/install"
examples_root="/tmp/openpit-cpp-examples"

rm -rf "${work_root}" "${examples_root}"
mkdir -p "${work_root}" "${install_dir}"

echo "==> Downloading ${asset}"
curl -fsSL "${base_url}/${asset}" -o "${work_root}/${asset}"
curl -fsSL "${base_url}/${asset}.sha256" -o "${work_root}/${asset}.sha256"

expected_sha="$(tr -d '[:space:]' < "${work_root}/${asset}.sha256")"
if command -v sha256sum >/dev/null 2>&1; then
  actual_sha="$(sha256sum "${work_root}/${asset}" | awk '{print $1}')"
else
  actual_sha="$(shasum -a 256 "${work_root}/${asset}" | awk '{print $1}')"
fi
if [[ "${actual_sha}" != "${expected_sha}" ]]; then
  echo "sha256 mismatch for ${asset}" >&2
  echo "expected: ${expected_sha}" >&2
  echo "actual:   ${actual_sha}" >&2
  exit 1
fi

tar -xzf "${work_root}/${asset}" -C "${install_dir}"

echo "==> Building minimal C++ consumer"
cmake -S /opt/e2e/cpp-consumer -B "${work_root}/consumer-build" \
  -DCMAKE_PREFIX_PATH="${install_dir}"
cmake --build "${work_root}/consumer-build" --parallel
"${work_root}/consumer-build/openpit_cpp_consumer"

mkdir -p "${examples_root}/cpp"
cp -R /opt/e2e/examples/. "${examples_root}/cpp"
cp -R /opt/e2e/tables "${examples_root}/tables"

run_example() {
  local name="$1"
  shift
  local src="${examples_root}/cpp/${name}"
  local build="${work_root}/examples/${name}"

  echo "==> Building C++ example ${name}"
  cmake -S "${src}" -B "${build}" \
    -DCMAKE_PREFIX_PATH="${install_dir}" \
    "$@"
  cmake --build "${build}" --parallel

  echo "==> Testing C++ example ${name}"
  ctest --test-dir "${build}" --output-on-failure
}

run_example "rate_pnl_killswitch" \
  -DOPENPIT_EXAMPLE_USE_INSTALLED_PACKAGE=ON
run_example "spot_funds"
run_example "spot_table" \
  -DOPENPIT_USE_FIND_PACKAGE=ON
run_example "spot_loadtest" \
  -DOPENPIT_USE_FIND_PACKAGE=ON
