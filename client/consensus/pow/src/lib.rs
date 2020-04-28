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

use {
    std::{fmt::Debug, marker::PhantomData, sync::Arc},
    futures::{Future, IntoFuture},
    log::warn,
    parking_lot::RwLock,
};
use sc_client_api::BlockchainEvents;
use sp_runtime::{RuntimeString, traits::{Block, Header, DigestItemFor, NumberFor}};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{BlockImport, Environment, Proposer, SyncOracle, import_queue::{BoxBlockImport, BoxJustificationImport}};
use sp_consensus::import_queue::{
    BasicQueue
};
use sp_inherents::InherentDataProviders;
use sp_core::crypto::Pair;
use codec::{Decode, Encode, Codec};
//use foreign_chain::{ForeignChain, ForeignChainConfig};
use yee_sharding_primitives::ShardingAPI;
use  pow_primitives::YeePOWApi;

pub use digest::CompatibleDigestItem;
pub use pow::{PowSeal, WorkProof, ProofNonce, ProofMulti,
              MiningAlgorithm, MiningHash, OriginalMerkleProof, CompactMerkleProof};
pub use job::{JobManager, DefaultJobManager, DefaultJob};
use yee_sharding::{ShardingDigestItem, ScaleOutPhaseDigestItem};
use yee_srml_pow::RewardCondition;
use yee_sharding_primitives::ScaleOut;
use sp_core::H256;
// use substrate_service::ServiceFactory;
use yee_context::Context;

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
    pub context: Context<B>
}

pub fn start_pow<B, P, C, I, E, AccountId, SO, OnExit>(
    local_key: Arc<P>,
    client: Arc<C>,
    block_import: Arc<I>,
    env: Arc<E>,
    sync_oracle: &'static mut SO,
    on_exit: OnExit,
    inherent_data_providers: InherentDataProviders,
    job_manager: Arc<RwLock<Option<Arc<dyn JobManager<Job=DefaultJob<B, P::Public>>>>>>,
    params: Params<AccountId, B>,
) -> Result<impl Future<Item=(), Error=()>, sp_consensus::Error> where
    B: Block,
    P: Pair + 'static,
    <P as Pair>::Public: Clone + Debug + Decode + Encode + Send + Sync,
    C: ProvideRuntimeApi<B> + HeaderBackend<B> + 'static,
    <C as ProvideRuntimeApi<B>>::Api: YeePOWApi<B>,
    I: BlockImport<B, Error=sp_consensus::Error> + Send + Sync + 'static,
    E: Environment<B> + Send + Sync + 'static,
    <E as Environment<B>>::Error: Debug + Send,
    //<<<E as Environment<B>>::Proposer as Proposer<B>>::Create as IntoFuture>::Future: Send + 'static,
    AccountId: Clone + Debug + Decode + Encode + Default + Send + Sync + 'static,
    SO: SyncOracle + Send + Sync + Clone,
    OnExit: Future<Item=(), Error=()>,
    DigestItemFor<B>: CompatibleDigestItem<B, P::Public> + ShardingDigestItem<u16> + ScaleOutPhaseDigestItem<NumberFor<B>, u16>,
    <B as Block>::Hash: From<H256> + Ord,
{
    let inner_job_manager = Arc::new(DefaultJobManager::new(
        client.clone(),
        env.clone(),
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
        },
        None => {
            *reg_lock = Some(inner_job_manager.clone());
        }
    }

    let worker: Arc<worker::DefaultWorker<B, C, I, job::DefaultJobManager<B, C, E, AccountId, <P as sp_core::crypto::Pair>::Public, I>, AccountId, <P as sp_core::crypto::Pair>::Public>> = Arc::new(worker::DefaultWorker::new(
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
pub type PowImportQueue<B, C> = BasicQueue<B, sp_api::TransactionFor<C, B>>;

pub trait TriggerExit: Send + Sync{
    fn trigger_restart(&self);
    fn trigger_stop(&self);
}

#[derive(Clone)]
pub struct ShardExtra<AccountId> {
    pub coinbase: AccountId,
    pub shard_num: u16,
    pub shard_count: u16,
    pub scale_out: Option<ScaleOut>,
    pub trigger_exit: Arc<dyn TriggerExit>,
}

/// Start import queue for POW consensus
pub fn import_queue<B, C, AccountId, AuthorityId>(
    block_import: BoxBlockImport<B, sp_api::TransactionFor<C, B>>,
    justification_import: Option<BoxJustificationImport<B>>,
    client: Arc<C>,
    inherent_data_providers: InherentDataProviders,
    //foreign_chains: Arc<RwLock<Option<ForeignChain<F>>>>,
    shard_extra: ShardExtra<AccountId>,
    context: Context<B>,
) -> Result<BasicQueue<B, sp_api::TransactionFor<C, B>>, sp_consensus::Error> where
    B: Block,
    H256: From<<B as Block>::Hash>,
    DigestItemFor<B>: CompatibleDigestItem<B, AuthorityId> + ShardingDigestItem<u16> + ScaleOutPhaseDigestItem<NumberFor<B>, u16>,
    C: ProvideRuntimeApi<B> + 'static + Send + Sync,
    C: HeaderBackend<B>,
    C: BlockchainEvents<B>,
    <C as ProvideRuntimeApi<B>>::Api: ShardingAPI<B> + YeePOWApi<B>,
    AccountId: Codec + Send + Sync + Clone + Default + 'static,
    AuthorityId: Decode + Encode + Clone + Send + Sync + 'static,
    <<<C as ProvideRuntimeApi<B>>::Api as sp_api::ApiExt<B>>::StateBackend as sp_state_machine::backend::Backend<<<B as Block>::Header as Header>::Hashing>>::Transaction: sp_api::ProvideRuntimeApi<B>
{
    register_inherent_data_provider(&inherent_data_providers, shard_extra.coinbase.clone())?;

    let verifier = Arc::new(
        verifier::PowVerifier {
            client,
            inherent_data_providers,
            //foreign_chains,
            phantom: PhantomData,
            shard_extra,
            context,
        }
    );
    Ok(BasicQueue::<B, sp_api::TransactionFor<C, B>>::new(verifier, block_import, justification_import, None))
}

pub fn register_inherent_data_provider<AccountId: 'static + Codec + Send + Sync>(
    inherent_data_providers: &InherentDataProviders,
    coinbase: AccountId,
) -> Result<(), sp_consensus::Error> where
    AccountId : Codec + Send + Sync + 'static, {

    if !inherent_data_providers.has_provider(&yee_srml_pow::INHERENT_IDENTIFIER) {
        inherent_data_providers.register_provider(yee_srml_pow::InherentDataProvider::new(coinbase, RewardCondition::Normal))
            .map_err(inherent_to_common_error)
    } else {
        Ok(())
    }
}

fn inherent_to_common_error(err: RuntimeString) -> sp_consensus::Error {
    sp_consensus::Error::InherentData(err.into()).into()
}
