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

use sp_inherents::{InherentData, InherentIdentifier, ProvideInherentData, InherentDataProviders};
use sp_std::result;
use codec::{Codec, Decode};

use crate::{PowInfo, RewardCondition};

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"YeePow00";

pub type InherentType<AccountId> = PowInfo<AccountId>;

pub trait PowInherentData<AccountId> {
	fn pow_inherent_data(&self) -> Result<InherentType<AccountId>, sp_inherents::Error>;
	fn pow_replace_inherent_data(&mut self, new: InherentType<AccountId>);
}

impl<AccountId: Codec> PowInherentData<AccountId> for InherentData {
	fn pow_inherent_data(&self) -> Result<InherentType<AccountId>, sp_inherents::Error> {
		self.get_data(&INHERENT_IDENTIFIER)
			.and_then(|r| r.ok_or_else(|| "YeePow inherent data not found".into()))
	}

	fn pow_replace_inherent_data(&mut self, new: InherentType<AccountId>) {
		self.replace_data(INHERENT_IDENTIFIER, &new);
	}
}

#[cfg(feature = "std")]
pub struct InherentDataProvider<AccountId> {
	pow_info: PowInfo<AccountId>,
}

#[cfg(feature = "std")]
impl<AccountId> InherentDataProvider<AccountId> {
	pub fn new(coinbase: AccountId, reward_condition: RewardCondition) -> Self {
		Self {
			pow_info: PowInfo { coinbase, reward_condition },
		}
	}
}

#[cfg(feature = "std")]
impl<AccountId: Codec> ProvideInherentData for InherentDataProvider<AccountId> {

	fn on_register(
		&self,
		providers: &InherentDataProviders,
	) -> result::Result<(), sp_inherents::Error> {
		if !providers.has_provider(&sp_timestamp::INHERENT_IDENTIFIER) {
			// Add the timestamp inherent data provider, as we require it.
			providers.register_provider(sp_timestamp::InherentDataProvider)
		} else {
			Ok(())
		}
	}

	fn inherent_identifier(&self) -> &'static [u8; 8] {
		&INHERENT_IDENTIFIER
	}

	fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), sp_inherents::Error> {
		inherent_data.put_data(INHERENT_IDENTIFIER, &self.pow_info)
	}

	fn error_to_string(&self, error: &[u8]) -> Option<String> {
		sp_inherents::Error::decode(&mut &error[..]).map(|e| e.into_string()).ok()
	}
}
