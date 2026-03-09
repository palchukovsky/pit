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

//! Pre-trade pipeline types and extension points.
//!
//! This module contains the request lifecycle used by Pit before an order is
//! sent to a venue:
//!
//! - [`CheckPreTradeStartPolicy`] models fast admission checks;
//! - [`Policy`] models deeper stateful checks;
//! - [`Request`] is the single-use handle returned after start-stage success;
//! - [`Reservation`] is the finalizable handle for reserved state;
//! - [`ExecutionReport`] feeds realized outcomes back into the engine.
//!
//! Custom controls typically start from the policy traits plus [`Context`] and
//! [`Mutations`].

mod context;
pub(crate) mod handles;
mod mutations;
pub mod policies;
mod policy;
mod reject;
mod report;
mod request;
mod reservation;
pub(crate) mod start_pre_trade_time;

pub use context::Context;
pub use mutations::{Mutation, Mutations, RiskMutation};
pub use policy::{CheckPreTradeStartPolicy, Policy};
pub use reject::{Reject, RejectCode, RejectScope};
pub use report::{ExecutionReport, PostTradeResult};
pub use request::Request;
pub use reservation::Reservation;
