// Copyright 2019, 2020 Wingchain
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

use codec::Decode;
use sp_inherents::{InherentData, InherentIdentifier, ProvideInherentData};

use crate::{ScaleOut, ShardInfo};

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"YeeShard";

pub type InherentType = ShardInfo<u16>;

pub trait YeeShardInherentData {
	fn yee_shard_inherent_data(&self) -> Result<InherentType, sp_inherents::Error>;
	fn yee_shard_replace_inherent_data(&mut self, new: InherentType);
}

impl YeeShardInherentData for InherentData {
	fn yee_shard_inherent_data(&self) -> Result<InherentType, sp_inherents::Error> {
		self.get_data(&INHERENT_IDENTIFIER)
			.and_then(|r| r.ok_or_else(|| "YeeShard inherent data not found".into()))
	}

	fn yee_shard_replace_inherent_data(&mut self, new: InherentType) {
		self.replace_data(INHERENT_IDENTIFIER, &new);
	}
}

#[cfg(feature = "std")]
pub struct InherentDataProvider {
	shard_info: ShardInfo<u16>,
}

#[cfg(feature = "std")]
impl InherentDataProvider {
	pub fn new(num: u16, count: u16, scale_out: Option<ScaleOut<u16>>) -> Self {
		Self {
			shard_info: ShardInfo { num, count, scale_out },
		}
	}
}

#[cfg(feature = "std")]
impl ProvideInherentData for InherentDataProvider {
	fn inherent_identifier(&self) -> &'static [u8; 8] {
		&INHERENT_IDENTIFIER
	}

	fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), sp_inherents::Error> {
		inherent_data.put_data(INHERENT_IDENTIFIER, &self.shard_info)
	}

	fn error_to_string(&self, error: &[u8]) -> Option<String> {
		sp_inherents::Error::decode(&mut &error[..]).map(|e| e.into_string()).ok()
	}
}
