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

use ansi_term::Colour;
use parking_lot::Mutex;
use sp_core::H256;

use {
	futures::{
		future::{self, Either},
		Future,
		prelude::*,
	},
	log::{info, warn},
	std::{
		fmt::Debug,
		marker::PhantomData,
		sync::{Arc, RwLock},
		time::{Duration, Instant},
	},
};
use {
	sp_consensus::{
		BlockImport, BlockImportParams,
		BlockOrigin, ForkChoiceStrategy, SyncOracle,
	},
	sp_inherents::InherentDataProviders,
	sp_runtime::{
		codec::{Codec, Decode, Encode},
		traits::{
			Block,
			DigestItemFor, Header, NumberFor,
		},
	},
};
use yc_sharding::{ScaleOutPhaseDigestItem, ShardingDigestItem};

use crate::job::{DefaultJob, JobManager};
use crate::pow::check_work_proof;
use crate::ShardExtra;
use crate::verifier::check_scale;
use futures_timer::Delay;

use super::{
	CompatibleDigestItem, ProofNonce, WorkProof,
};
use std::pin::Pin;
use futures::task::{Context, Poll};

pub trait PowWorker<JM: JobManager> {
	type Error: Debug + Send;
	type OnJob: Future<Output=Result<JM::Job, Self::Error>>;
	type OnWork: Future<Output=Result<(), Self::Error>>;

	fn stop_sign(&self) -> Arc<RwLock<bool>>;

	fn on_start(&self) -> Result<(), Self::Error>;

	fn on_job(&self) -> Self::OnJob;

	fn on_work(&self, iter: u64) -> Self::OnWork;
}

pub struct DefaultWorker<B, I, JM, AccountId, AuthorityId> {
	job_manager: Arc<JM>,
	block_import: Arc<Mutex<I>>,
	inherent_data_providers: InherentDataProviders,
	stop_sign: Arc<RwLock<bool>>,
	shard_extra: ShardExtra<AccountId>,
	phantom: PhantomData<(B, AuthorityId)>,
}

impl<B, I, JM, AccountId, AuthorityId> DefaultWorker<B, I, JM, AccountId, AuthorityId> where
	B: Block,
	JM: JobManager,
{
	pub fn new(
		job_manager: Arc<JM>,
		block_import: Arc<Mutex<I>>,
		inherent_data_providers: InherentDataProviders,
		shard_extra: ShardExtra<AccountId>,
	) -> Self {
		DefaultWorker {
			job_manager,
			block_import,
			inherent_data_providers,
			stop_sign: Default::default(),
			shard_extra,
			phantom: PhantomData,
		}
	}
}

impl<B, I, JM, AccountId, AuthorityId> PowWorker<JM> for DefaultWorker<B, I, JM, AccountId, AuthorityId> where
	B: Block,
	I: BlockImport<B, Error=sp_consensus::Error> + Send + Sync + 'static,
	DigestItemFor<B>: CompatibleDigestItem<B, AuthorityId> + ShardingDigestItem<u16> + ScaleOutPhaseDigestItem<NumberFor<B>, u16>,
	JM: JobManager<Job=DefaultJob<B, AuthorityId>>,
	AccountId: Codec + Send + Sync + Clone + 'static,
	AuthorityId: Decode + Encode + Send + Sync + Clone + 'static,
	AuthorityId: Decode + Encode + Clone + 'static,
	B::Hash: From<H256> + Ord,
{
	type Error = sp_consensus::Error;
	type OnJob = Pin<Box<dyn Future<Output=Result<DefaultJob<B, AuthorityId>, Self::Error>> + Send>>;
	type OnWork = Pin<Box<dyn Future<Output=Result<(), Self::Error>> + Send>>;

	fn stop_sign(&self) -> Arc<RwLock<bool>> {
		self.stop_sign.clone()
	}

	fn on_start(&self) -> Result<(), sp_consensus::Error> {
		super::register_inherent_data_provider(&self.inherent_data_providers, self.shard_extra.coinbase.clone())
	}

	fn on_job(&self) -> Self::OnJob {
		self.job_manager.get_job()
	}

	fn on_work(&self,
			   iter: u64,
	) -> Self::OnWork {
		let mut block_import = self.block_import.clone();

		let job = self.on_job().into_future();

		let shard_extra = self.shard_extra.clone();

		let on_proposal_block = move |job: DefaultJob<B, AuthorityId>| -> Result<(), sp_consensus::Error> {
			let header = job.header;
			let body = job.body;
			let header_num = header.number().clone();
			let header_pre_hash = header.hash();
			let digest_item = job.digest_item;
			let pow_target = digest_item.pow_target;
			let xts_proof = job.xts_proof.clone();

			info!("block template {} @ {:?}, pow target: {:#x}", header_num, header_pre_hash, pow_target);

			// TODO: remove hardcoded
			const PREFIX: &str = "yeeroot-";

			for i in 0_u64..iter {
				let shard_extra = shard_extra.clone();
				let proof = WorkProof::Nonce(ProofNonce::get_with_prefix_len(PREFIX, 12, i));
				let mut seal = digest_item.clone();
				seal.work_proof = proof;

				if let Ok((post_digest, hash)) = check_work_proof(&header, &seal) {
					check_scale::<B, AccountId>(&header, shard_extra)?;

					let mut import_block = BlockImportParams::new(BlockOrigin::Own, header);
					import_block.post_digests.push(post_digest);
					import_block.body = Some(body);
					import_block.storage_changes = None;//TODO Some(storage_changes);
					import_block.fork_choice = Some(ForkChoiceStrategy::LongestChain);

					block_import.import_block(import_block, Default::default())?;

					info!("{} @ {} {:?}", Colour::Green.bold().paint("Block mined"), header_num, hash);
					return Ok(());
				}
			}

			Ok(())
		};

		Box::new(
			job
				.map_err(|e| {
					warn!("job error: {:?}", e);
					e
				})
				.map_err(to_common_error)
				.map(move |job| {
					if let Err(e) = on_proposal_block(job) {
						warn!("block proposal failed {:?}", e);
					}
				})
		)
	}
}

pub fn to_common_error<E: Debug>(e: E) -> sp_consensus::Error {
	sp_consensus::Error::ClientImport(format!("{:?}", e)).into()
}

pub fn start_worker<W, SO, JM, OnExit>(
	worker: Arc<W>,
	sync_oracle: SO,
	on_exit: OnExit,
	mine: bool,
) -> Result<impl Future<Output=()>, sp_consensus::Error> where
	W: PowWorker<JM>,
	SO: SyncOracle,
	JM: JobManager,
	OnExit: Future<Output=()>,
{
	worker.on_start().map_err(to_common_error)?;

	let stop_sign = worker.stop_sign();

	info!("worker loop start");
	let work = loop_fn((), move |()| {
		let delay = Delay::new(Duration::from_secs(5));
		let delayed_continue = Either::Left(delay.then(|_| future::ok(Loop::Continue(()))));
		let no_delay_stop = Either::Right(future::ok(Loop::Break(())));

		if !mine {
			return Either::Left(no_delay_stop);
		}

		match worker.stop_sign().read() {
			Ok(stop_sign) => {
				if *stop_sign {
					return Either::Left(no_delay_stop);
				}
			}
			Err(e) => {
				warn!("work stop sign read error {:?}", e);
				return Either::Left(no_delay_stop);
			}
		}

		// worker main loop
		info!("worker one loop start");

		if sync_oracle.is_major_syncing() {
			return Either::Left(delayed_continue);
		}

		let task = worker.on_work(10000).into_future();
		Either::Right(
			task.then(|_| Delay::new(Duration::from_secs(0)))
				.then(|_| future::ok(Loop::Continue(())))
		)
	});

	Ok(work.select(on_exit).then(move |_| {
		stop_sign.write()
			.map(|mut sign| { *sign = true; })
			.unwrap_or_else(|e| { warn!("write stop sign error : {:?}", e); });

		future::ready(())
	}))
}

#[derive(Debug)]
pub(crate) enum Loop<T, S> {
	/// Indicates that the loop has completed with output `T`.
	Break(T),

	/// Indicates that the loop function should be called again with input
	/// state `S`.
	Continue(S),
}

/// Created by the `loop_fn` function.
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub(crate) struct LoopFn<A, F> {
	future: A,
	func: F,
}

/// Creates a new future implementing a tail-recursive loop.
pub(crate) fn loop_fn<S, T, A, F, E>(initial_state: S, mut func: F) -> LoopFn<A, F>
	where
		F: FnMut(S) -> A,
		A: Future<Output=Result<Loop<T, S>, E>>,
{
	LoopFn {
		future: func(initial_state),
		func,
	}
}

impl<S, T, A, F, E> Future for LoopFn<A, F>
	where
		F: FnMut(S) -> A,
		A: Future<Output=Result<Loop<T, S>, E>>,
{
	type Output = Result<T, E>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<T, E>> {
		loop {
			unsafe {
				let this = Pin::get_unchecked_mut(self);

				match Pin::new_unchecked(&mut this.future).poll(cx) {
					Poll::Ready(t) => match t {
						Ok(Loop::Break(x)) => return Poll::Ready(Ok(x)),
						Ok(Loop::Continue(s)) => this.future = (this.func)(s),
						Err(e) => return Poll::Ready(Err(e)),
					},
					Poll::NotReady => return Poll::NotReady,
				}
				self = Pin::new_unchecked(this);
			}
		}
	}
}
