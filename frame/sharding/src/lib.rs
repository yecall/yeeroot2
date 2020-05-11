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

use frame_support::Parameter;

#[cfg(feature = "std")]
use {
	serde::Serialize,
};
use {
	frame_support::{
		decl_module, decl_storage,
		storage::StorageValue,
	},
	frame_system::{self as system, ensure_none},
	sp_arithmetic::traits::BaseArithmetic,
	sp_inherents::{
		InherentData, InherentIdentifier,
		MakeFatalError, ProvideInherent,
	},
	sp_runtime::{
		codec::{
			Codec, Decode, Encode,
		},
		traits::{
			MaybeDisplay,
			MaybeSerializeDeserialize, Member,
		},
	},
	sp_std::fmt::Debug,
	yp_sharding::{ShardInfo, ShardingInfo},
	yp_sharding::inherents::INHERENT_IDENTIFIER
};

pub type Log<T> = RawLog<<T as Trait>::ShardNum, <T as system::Trait>::BlockNumber>;

/// Logs in this module.
#[cfg_attr(feature = "std", derive(Serialize, Debug))]
#[derive(Encode, Decode, PartialEq, Eq, Clone)]
pub enum RawLog<ShardNum, BlockNumber> {
	/// Block Header digest log for shard info
	ShardMarker(ShardNum, ShardNum),
	ScaleOutPhase(ScaleOutPhase<BlockNumber, ShardNum>),
}

pub trait Trait: system::Trait {
	/// Type for shard number
	type ShardNum: Parameter + Member + MaybeSerializeDeserialize + Debug + Default + Copy + MaybeDisplay + BaseArithmetic + Codec;
	/// Type for all log entries of this module.
	type Log: From<Log<Self>> + Into<system::DigestItemOf<Self>>;
}

/*
#[cfg(any(feature = "std", test))]
impl<N> From<RawLog<N>> for runtime_primitives::testing::DigestItem {
    fn from(log: RawLog<N>) -> Self {
        match log {
            RawLog::ShardMarker(shard) => {
                runtime_primitives::generic::DigestItem::Other(format!("YeeShard: {:?}", shard).encode())
            }
        }
    }
}
*/

#[derive(Clone, PartialEq, Eq)]
#[derive(Decode, Encode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize))]
pub enum ScaleOutPhase<BlockNumber, ShardNum> {
	Started {
		observe_util: BlockNumber,
		shard_num: ShardNum,
	},
	NativeReady {
		observe_util: BlockNumber,
		shard_num: ShardNum,
	},
	Ready {
		observe_util: BlockNumber,
		shard_num: ShardNum,
	},
	Commiting {
		shard_count: ShardNum,
	},
	Committed {
		shard_num: ShardNum,
		shard_count: ShardNum,
	},
}

decl_storage! {
    trait Store for Module<T: Trait> as Sharding {
        /// Total sharding count used in genesis block
        pub GenesisShardingCount get(fn genesis_sharding_count) config(): T::ShardNum;

        /// Total sharding count used in genesis block
        pub ScaleOutObserveBlocks get(fn scale_out_observe_blocks) config(): T::BlockNumber;

        /// Storage for ShardInfo used for current block
        pub CurrentShardInfo get(fn current_shard_info): Option<ShardInfo<T::ShardNum>>;

        /// Storage for ScaleOutPhase used for current block
        pub CurrentScaleOutPhase get(fn current_scale_out_phase): Option<ScaleOutPhase<T::BlockNumber, T::ShardNum>>;

    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        #[weight = 0]
        fn set_shard_info(origin, info: ShardInfo<T::ShardNum>) {
            ensure_none(origin)?;

            let info_clone = info.clone();
            <Self as Store>::CurrentShardInfo::mutate(|orig| {
                *orig = Some(info_clone);
            });

            let block_number = <system::Module<T>>::block_number();
            let scale_out_observe_blocks = Self::scale_out_observe_blocks();

            let current_scale_out_phase = Self::current_scale_out_phase();

            let target_shard_num = match info.scale_out.clone(){
                Some(scale_out) => scale_out.shard_num,
                None => info.num,
            };

            match current_scale_out_phase {
                None => {
                    if let Some(_) = info.scale_out {
                        <Self as Store>::CurrentScaleOutPhase::mutate(|orig| {
                            *orig = Some(ScaleOutPhase::Started{
                                observe_util: block_number + scale_out_observe_blocks,
                                shard_num: target_shard_num,
                            });
                        });
                    }
                },
                Some(current_scale_out_phase) => match current_scale_out_phase{
                    ScaleOutPhase::Started{observe_util, ..} => {

                        //TODO: check scaled shard_num percentage
                        if observe_util == block_number{
                            <Self as Store>::CurrentScaleOutPhase::mutate(|orig| {
                                *orig = Some(ScaleOutPhase::NativeReady{
                                    observe_util: block_number + scale_out_observe_blocks,
                                    shard_num: target_shard_num,
                                });
                            });
                        }
                    },
                    ScaleOutPhase::NativeReady{observe_util, ..} => {

                        //TODO: check foreign scale out phase
                        if observe_util == block_number{
                            <Self as Store>::CurrentScaleOutPhase::mutate(|orig| {
                                *orig = Some(ScaleOutPhase::Ready{
                                    observe_util: block_number + scale_out_observe_blocks,
                                    shard_num: target_shard_num,
                                });
                            });
                        }

                    },
                    ScaleOutPhase::Ready{observe_util, ..} => {

                        if observe_util == block_number{

                            let scale_out_shard_count = info.count + info.count;

                            <Self as Store>::CurrentScaleOutPhase::mutate(|orig| {
                                *orig = Some(ScaleOutPhase::Commiting{
                                    shard_count: scale_out_shard_count,
                                });
                            });
                        }
                    },
                    ScaleOutPhase::Commiting{shard_count} => {

                        <Self as Store>::CurrentScaleOutPhase::mutate(|orig| {
                            *orig = Some(ScaleOutPhase::Committed{
                                shard_num: target_shard_num,
                                shard_count: shard_count,
                            });
                        });
                    },
                    ScaleOutPhase::Committed{..} => {

                        <Self as Store>::CurrentScaleOutPhase::mutate(|orig| {
                            *orig = None;
                        });
                    },
                }
            }

        }

        fn on_finalize(_block_number: T::BlockNumber) {

            if let Some(shard_info) = Self::current_shard_info() {
                Self::deposit_log(RawLog::ShardMarker(shard_info.num, shard_info.count));
            }

            if let Some(scale_out_phase) = Self::current_scale_out_phase() {
                Self::deposit_log(RawLog::ScaleOutPhase(scale_out_phase));
            }
        }
    }
}

impl<T: Trait> Module<T> {
	/// Deposit one of this module's logs.
	fn deposit_log(log: Log<T>) {
		<system::Module<T>>::deposit_log(<T as Trait>::Log::from(log).into());
	}
}

impl<T: Trait> ShardingInfo<T::ShardNum> for Module<T> {
	fn get_genesis_shard_count() -> <T as Trait>::ShardNum {
		Self::genesis_sharding_count()
	}

	fn get_curr_shard() -> Option<T::ShardNum> {
		Some(Self::current_shard_info()
			.expect("shard info must be ready for runtime modules")
			.num
		)
	}

	fn get_shard_count() -> T::ShardNum {
		Self::current_shard_info()
			.expect("shard info must be ready for runtime modules")
			.count
	}
}

impl<T: Trait> ProvideInherent for Module<T> {
	type Call = Call<T>;
	type Error = MakeFatalError<sp_inherents::Error>;
	const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

	fn create_inherent(data: &InherentData) -> Option<Self::Call> {
		let data = extract_inherent_data::<T::ShardNum>(data)
			.expect("Sharding inherent data must exist");

		Some(Call::set_shard_info(data))
	}

	fn check_inherent(_: &Self::Call, _: &InherentData) -> Result<(), Self::Error> {
		Ok(())
	}
}

fn extract_inherent_data<N>(data: &InherentData) -> Result<ShardInfo<N>, sp_inherents::Error> where
	N: Decode,
{
	data.get_data::<ShardInfo<N>>(&INHERENT_IDENTIFIER)
		.map_err(|_| sp_inherents::Error::from("Invalid sharding inherent data encoding."))?
		.ok_or_else(|| "Sharding inherent data is not provided.".into())
}
