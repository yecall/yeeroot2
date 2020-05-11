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

use std::time::Duration;

use ansi_term::Colour;
use codec::{Decode, Encode};
use log::info;
use sp_core::H256;

use {
	futures::{
		Future, future,
	},
	log::warn,
	std::{
		fmt::Debug,
		marker::PhantomData,
		sync::Arc,
		time::{
			SystemTime, UNIX_EPOCH,
		},
	},
};
use {
	sp_api::ProvideRuntimeApi,
	sp_blockchain::HeaderBackend,
	sp_consensus::{
		BlockImport, BlockImportParams, BlockOrigin, Environment, ForkChoiceStrategy, Proposer,
	},
	sp_inherents::InherentDataProviders,
	sp_runtime::{
		traits::{Block, DigestItemFor, NumberFor, Header},
	},
};
use {
	super::{
		worker::to_common_error,
	},
};
use yc_sharding::{ScaleOutPhaseDigestItem, ShardingDigestItem};
use yp_consensus_pow::YeePOWApi;
use yp_context::Context;

use crate::{CompatibleDigestItem, PowSeal, ShardExtra, WorkProof};
use crate::pow::{calc_pow_target, check_work_proof, gen_extrinsic_proof};
use crate::verifier::check_scale;
use parking_lot::Mutex;
use sp_consensus::{SelectChain, RecordProof};
use sp_runtime::Digest;

#[derive(Clone)]
pub struct DefaultJob<B: Block, AuthorityId: Decode + Encode + Clone> {
	/// Hash for header with consensus post-digests (unknown WorkProof) applied
	/// The hash has 2 uses:
	/// 1. distinguish different job
	/// 2. build merkle tree of ProofMulti
	pub hash: B::Hash,
	/// The header, without consensus post-digests applied
	pub header: B::Header,
	/// Block's body
	pub body: Vec<B::Extrinsic>,
	/// Digest item
	pub digest_item: PowSeal<B, AuthorityId>,
	/// extrinsic proof
	pub xts_proof: Vec<u8>,
}

impl<B: Block, AuthorityId: Decode + Encode + Clone> Job for DefaultJob<B, AuthorityId> {
	type Hash = B::Hash;
}

pub trait Job {
	type Hash;
}

pub trait JobManager: Send + Sync
{
	type Job: Job;

	/// get job with unknown proof
	fn get_job(&mut self) -> Box<dyn Future<Output=Result<Self::Job, sp_consensus::Error>> + Send>;

	/// submit job
	fn submit_job(&self, job: Self::Job) -> Box<dyn Future<Output=Result<<Self::Job as Job>::Hash, sp_consensus::Error>> + Send>;
}

pub struct DefaultJobManager<B, C, SC, E, AccountId, AuthorityId, I> where
	B: Block,
{
	client: Arc<C>,
	select_chain: SC,
	env: E,
	inherent_data_providers: InherentDataProviders,
	authority_id: AuthorityId,
	block_import: Arc<Mutex<I>>,
	shard_extra: ShardExtra<AccountId>,
	context: Context<B>,
	phantom: PhantomData<B>,
}

impl<B, C, SC, E, AccountId, AuthorityId, I> DefaultJobManager<B, C, SC, E, AccountId, AuthorityId, I> where
	B: Block,
	E: Environment<B> + 'static,
	<E as Environment<B>>::Proposer: Proposer<B>,
	<E as Environment<B>>::Error: Debug,
	AuthorityId: Decode + Encode + Clone,
	I: BlockImport<B, Error=sp_consensus::Error> + Send + Sync + 'static,
{
	pub fn new(
		client: Arc<C>,
		select_chain: SC,
		env: E,
		inherent_data_providers: InherentDataProviders,
		authority_id: AuthorityId,
		block_import: Arc<Mutex<I>>,
		shard_extra: ShardExtra<AccountId>,
		context: Context<B>,
	) -> Self {
		Self {
			client,
			select_chain,
			env,
			inherent_data_providers,
			authority_id,
			block_import,
			shard_extra,
			context,
			phantom: PhantomData,
		}
	}
}

impl<B, C, SC, E, AccountId, AuthorityId, I> JobManager for DefaultJobManager<B, C, SC, E, AccountId, AuthorityId, I>
	where B: Block,
		  DigestItemFor<B>: super::CompatibleDigestItem<B, AuthorityId> + ShardingDigestItem<u16> + ScaleOutPhaseDigestItem<NumberFor<B>, u16>,
		  C: HeaderBackend<B> + ProvideRuntimeApi<B>,
		  C::Api: YeePOWApi<B>,
		  SC: SelectChain<B> + Send + Sync,
		  E: Environment<B> + Send + Sync + 'static,
		  E::Proposer: Proposer<B, Transaction=sp_api::TransactionFor<C, B>>,
		  E::Error: Debug,
		  AuthorityId: Decode + Encode + Clone + Send + Sync + 'static,
		  AccountId: Decode + Encode + Clone + Send + Sync + 'static,
		  I: BlockImport<B, Error=sp_consensus::Error> + Send + Sync + 'static,
		  B::Hash: From<H256> + Ord,
{
	type Job = DefaultJob<B, AuthorityId>;

	fn get_job(&mut self) -> Box<dyn Future<Output=Result<Self::Job, sp_consensus::Error>> + Send> {
		let chain_head = match self.select_chain.best_chain()
			.map_err(to_common_error) {
			Ok(chain_head) => chain_head,
			Err(e) => return Box::new(future::err(e)),
		};

		let inherent_data = match self.inherent_data_providers.create_inherent_data()
			.map_err(to_common_error) {
			Ok(inherent_data) => inherent_data,
			Err(e) => return Box::new(future::err(e)),
		};

		let awaiting_proposer = self.env.init(&chain_head);

		let client = self.client.clone();
		let authority_id = self.authority_id.clone();
		let context = self.context.clone();

		let build_job = move |block: B| {
			let (header, body) = block.deconstruct();
			let header_num = header.number().clone();
			let header_pre_hash = header.hash();
			let timestamp = timestamp_now()?;
			let pow_target = calc_pow_target(client, &header, timestamp, &context)?;
			let authority_id = authority_id;
			let work_proof = WorkProof::Unknown;
			// generate proof
			let (relay_proof, proof) = gen_extrinsic_proof::<B>(&header, &body);

			let pow_seal = PowSeal {
				authority_id,
				pow_target,
				timestamp,
				work_proof,
				relay_proof,
			};
			let mut header_with_pow_seal = header.clone();
			let item = <DigestItemFor<B> as CompatibleDigestItem<B, AuthorityId>>::pow_seal(pow_seal.clone());
			header_with_pow_seal.digest_mut().push(item);

			let hash = header_with_pow_seal.hash();

			info!("job {} @ {:?}, pow target: {:#x}", header_num, header_pre_hash, pow_target);

			Ok(DefaultJob {
				hash,
				header,
				body,
				digest_item: pow_seal,
				xts_proof: proof,
			})
		};

		awaiting_proposer.and_then(move |mut proposer| proposer.propose(
			inherent_data,
			Digest::default(),
			Duration::from_secs(10),
			RecordProof::No,
		)).and_then(build_job)


		// Box::new(awaiting_proposer.and_then(inherent_data, Duration::from_secs(10)).into_future()
		// 	.map_err(to_common_error).and_then(build_job))
	}

	fn submit_job(&self, job: Self::Job) -> Box<dyn Future<Output=Result<<Self::Job as Job>::Hash, sp_consensus::Error>> + Send> {
		let mut block_import = self.block_import.clone();

		let check_job = move |job: Self::Job| -> Result<<Self::Job as Job>::Hash, sp_consensus::Error>{
			let number = &job.header.number().clone();
			let (post_digest, hash) = check_work_proof(&job.header, &job.digest_item)?;

			check_scale::<B, AccountId>(&job.header, self.shard_extra.clone())?;

			let mut import_block = BlockImportParams::new(BlockOrigin::Own, job.header);
			import_block.post_digests.push(post_digest);
			import_block.body = Some(job.body);
			import_block.storage_changes = None;//TODO Some(storage_changes);
			import_block.fork_choice = Some(ForkChoiceStrategy::LongestChain);

			block_import.import_block(import_block, Default::default())?;
			info!("{} @ {} {:?}", Colour::Green.bold().paint("Block mined"), number, hash);
			Ok(hash)
		};

		Box::new(check_job(job).into_future())
	}
}

fn timestamp_now() -> Result<u64, sp_consensus::Error> {
	Ok(SystemTime::now().duration_since(UNIX_EPOCH)
		.map_err(to_common_error)?.as_millis() as u64)
}
