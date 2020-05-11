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

//! POW (Proof of Work) consensus in YeeChain

use sp_consensus::{BlockImport, Environment, SyncOracle, SelectChain, Proposer};
use sp_consensus::import_queue::{BasicQueue, BoxFinalityProofImport, BoxJustificationImport};
use sp_core::crypto::Pair;
use sp_core::H256;

use {
	futures::Future,
	log::warn,
	parking_lot::RwLock,
	std::{fmt::Debug, marker::PhantomData, sync::Arc},
};
use {
	// foreign_chain::{ForeignChain, ForeignChainConfig},
	sp_api::ProvideRuntimeApi,
	sp_blockchain::HeaderBackend,
	sp_inherents::InherentDataProviders,
	sp_runtime::{
		codec::{
			Codec, Decode, Encode,
		},
		traits::{
			Block,
			DigestItemFor,
			NumberFor,
		},
	},
	yp_sharding::ShardingAPI,
};
use {
	yp_consensus_pow::YeePOWApi,
};
pub use digest::CompatibleDigestItem;
pub use job::{DefaultJob, DefaultJobManager, JobManager};
pub use pow::{CompactMerkleProof, MiningAlgorithm, MiningHash, OriginalMerkleProof,
			  PowSeal, ProofMulti, ProofNonce, WorkProof};
use yc_sharding::{ScaleOutPhaseDigestItem, ShardingDigestItem};
use yp_consensus_pow::RewardCondition;
use yp_context::Context;
use yp_sharding::ScaleOut;
use parking_lot::Mutex;

mod job;
mod digest;
mod pow;
mod verifier;
mod worker;

pub struct Params<AccountId, B> where
	B: Block,
{
	pub force_authoring: bool,
	pub mine: bool,
	pub shard_extra: ShardExtra<AccountId>,
	pub context: Context<B>,
}

pub fn start_pow<B, P, C, SC, I, E, AccountId, SO, OnExit>(
	local_key: Arc<P>,
	client: Arc<C>,
	select_chain: SC,
	block_import: I,
	env: E,
	sync_oracle: SO,
	on_exit: OnExit,
	inherent_data_providers: InherentDataProviders,
	job_manager: Arc<RwLock<Option<Arc<dyn JobManager<Job=DefaultJob<B, P::Public>>>>>>,
	params: Params<AccountId, B>,
) -> Result<impl Future<Output=()>, sp_consensus::Error> where
	B: Block,
	P: Pair + 'static,
	<P as Pair>::Public: Clone + Debug + Decode + Encode + Send + Sync,
	C: HeaderBackend<B> + ProvideRuntimeApi<B> + 'static,
	C::Api: YeePOWApi<B>,
	SC: SelectChain<B> + 'static,
	I: BlockImport<B, Error=sp_consensus::Error> + Send + Sync + 'static,
	E: Environment<B> + Send + Sync + 'static,
	E::Proposer: Proposer<B, Transaction=sp_api::TransactionFor<C, B>>,
	E::Error: Debug + Send,
	AccountId: Clone + Debug + Decode + Encode + Default + Send + Sync + 'static,
	SO: SyncOracle + Send + Sync + Clone,
	OnExit: Future<Output=()>,
	DigestItemFor<B>: CompatibleDigestItem<B, P::Public> + ShardingDigestItem<u16> + ScaleOutPhaseDigestItem<NumberFor<B>, u16>,
	B::Hash: From<H256> + Ord,
{
	let block_import = Arc::new(Mutex::new(block_import));
	let inner_job_manager = Arc::new(DefaultJobManager::new(
		client.clone(),
		select_chain,
		env,
		inherent_data_providers.clone(),
		local_key.public(),
		block_import.clone(),
		params.shard_extra.clone(),
		params.context.clone(),
	));

	let mut reg_lock = job_manager.write();
	match *reg_lock {
		Some(_) => {
			warn!("job manager already registered");
			panic!("job manager can only be registered once");
		}
		None => {
			*reg_lock = Some(inner_job_manager.clone());
		}
	}

	let worker = Arc::new(worker::DefaultWorker::new(
		inner_job_manager.clone(),
		block_import,
		inherent_data_providers.clone(),
		params.shard_extra.clone(),
	));
	worker::start_worker(
		worker,
		sync_oracle,
		on_exit,
		params.mine)
}

/// POW chain import queue
pub type PowImportQueue<B, Transaction> = BasicQueue<B, Transaction>;


pub trait TriggerExit: Send + Sync {
	fn trigger_restart(&self);
	fn trigger_stop(&self);
}

#[derive(Clone)]
pub struct ShardExtra<AccountId> {
	pub coinbase: AccountId,
	pub shard_num: u16,
	pub shard_count: u16,
	pub scale_out: Option<ScaleOut<u16>>,
	pub trigger_exit: Arc<dyn TriggerExit>,
}

/// Start import queue for POW consensus
pub fn import_queue<B, I, C, S, AccountId, AuthorityId>(
	block_import: I,
	justification_import: Option<BoxJustificationImport<B>>,
	finality_proof_import: Option<BoxFinalityProofImport<B>>,
	client: Arc<C>,
	inherent_data_providers: InherentDataProviders,
	// foreign_chains: Arc<RwLock<Option<ForeignChain<F>>>>,
	shard_extra: ShardExtra<AccountId>,
	context: Context<B>,
	spawner: &S,
) -> Result<PowImportQueue<B, sp_api::TransactionFor<C, B>>, sp_consensus::Error> where
	B: Block,
	H256: From<B::Hash>,
	DigestItemFor<B>: CompatibleDigestItem<B, AuthorityId> + ShardingDigestItem<u16> + ScaleOutPhaseDigestItem<NumberFor<B>, u16>,
	C: ProvideRuntimeApi<B> + 'static + Send + Sync,
	C: HeaderBackend<B>,
	C::Api: ShardingAPI<B> + YeePOWApi<B>,
	AccountId: Codec + Send + Sync + Clone + Default + 'static,
	AuthorityId: Decode + Encode + Clone + Send + Sync + 'static,
	I: BlockImport<B, Error=sp_consensus::Error, Transaction=sp_api::TransactionFor<C, B>> + Send + Sync + 'static,
	S: sp_core::traits::SpawnBlocking,
{
	register_inherent_data_provider(&inherent_data_providers, shard_extra.coinbase.clone())?;

	let verifier = verifier::PowVerifier {
		client,
		inherent_data_providers,
		// foreign_chains,
		phantom: PhantomData,
		shard_extra,
		context,
	};
	Ok(BasicQueue::new(
		verifier,
		Box::new(block_import),
		justification_import,
		finality_proof_import,
		spawner,
	))
}

pub fn register_inherent_data_provider<AccountId: 'static + Codec + Send + Sync>(
	inherent_data_providers: &InherentDataProviders,
	coinbase: AccountId,
) -> Result<(), sp_consensus::Error> where
	AccountId: Codec + Send + Sync + 'static, {
	if !inherent_data_providers.has_provider(&yp_consensus_pow::inherents::INHERENT_IDENTIFIER) {
		inherent_data_providers.register_provider(yp_consensus_pow::inherents::InherentDataProvider::new(coinbase, RewardCondition::Normal))
			.map_err(inherent_to_common_error)
	} else {
		Ok(())
	}
}

fn inherent_to_common_error(err: sp_inherents::Error) -> sp_consensus::Error {
	sp_consensus::Error::InherentData(err).into()
}
