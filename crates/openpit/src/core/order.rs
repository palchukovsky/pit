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
// Please see https://github.com/openpitkit and the OWNERS file for details.

use crate::param::{Price, Quantity, Side};

use super::Instrument;

/// Limit order submitted for pre-trade checks.
///
/// Market orders are not supported in this version, therefore `price` is
/// mandatory and not optional.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Order {
    /// Instrument being traded.
    pub instrument: Instrument,
    /// Trade side.
    pub side: Side,
    /// Order size in underlying asset units.
    pub quantity: Quantity,
    /// Limit price.
    pub price: Price,
}

#[cfg(test)]
mod tests {
    use crate::param::{Asset, Price, Quantity, Side};

    use super::Order;
    use crate::core::Instrument;

    #[test]
    fn order_exposes_required_fields() {
        let order = Order {
            instrument: Instrument::new(
                Asset::new("SPX").expect("asset code must be valid"),
                Asset::new("USD").expect("asset code must be valid"),
            ),
            side: Side::Sell,
            quantity: Quantity::from_str("2").expect("quantity must be valid"),
            price: Price::from_str("5000").expect("price must be valid"),
        };

        assert_eq!(
            order.instrument.underlying_asset(),
            &Asset::new("SPX").expect("asset code must be valid")
        );
        assert_eq!(order.side, Side::Sell);
    }
}
