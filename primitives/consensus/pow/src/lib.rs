// Copyright (C) 2019 Yee Foundation.
//
// This file is part of YeeChain.
//
// YeeChain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// YeeChain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with YeeChain.  If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use serde::Serialize;

use {
    sp_api::decl_runtime_apis,
    sp_runtime::{
        ConsensusEngineId,
        traits::NumberFor,
    },
};

///! Primitives for Yee POW

pub mod inherents;

/// `ConsensusEngineId` of Yee POW consensus.
pub const YEE_POW_ENGINE_ID: ConsensusEngineId = [b'Y', b'e', b'e', b'!'];

pub type PowTarget = sp_core::U256;

decl_runtime_apis! {
    pub trait YeePOWApi {
        /// POW target config used for genesis block
        fn genesis_pow_target() -> PowTarget;

        /// In-Chain config for POW target adjust period
        fn pow_target_adj() -> NumberFor<Block>;

        /// Target block time in seconds
        fn target_block_time() -> u64;
    }
}

#[derive(Clone, PartialEq, Eq)]
#[derive(Decode, Encode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize))]
pub struct PowInfo<AccountId> {
    pub coinbase: AccountId,
    pub reward_condition: RewardCondition,
}

#[derive(Clone, PartialEq, Eq)]
#[derive(Decode, Encode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize))]
pub enum RewardCondition {
    Normal,
    Slash,//TODO: provide slash reason
}
