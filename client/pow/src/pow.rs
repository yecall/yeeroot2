use sp_core::{H256, U256};
use sc_consensus_pow::{PowAlgorithm, Error};
use codec::{Encode, Decode};
use sp_runtime::generic::BlockId;
use sp_runtime::traits::Block as BlockT;
use sp_consensus_pow::Seal as RawSeal;
use rand::{thread_rng, SeedableRng, rngs::SmallRng};
use sha3::{Sha3_256, Digest};
use std::time::Duration;
// use std::sync::Arc;

pub type Difficulty = U256;

fn is_valid_hash(_hash: &H256, _difficulty: Difficulty) -> bool {
    true
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, Debug)]
pub struct Seal {
    pub difficulty: Difficulty,
    pub work: H256,
    pub nonce: H256,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, Debug)]
pub struct Calculation {
    pub difficulty: Difficulty,
    pub pre_hash: H256,
    pub nonce: H256,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Compute {
    pub pre_hash: H256,
    pub difficulty: Difficulty,
    pub nonce: H256,
}

impl Compute {
    pub fn compute(self) -> Seal {
        let calculation = Calculation {
            difficulty: self.difficulty,
            pre_hash: self.pre_hash,
            nonce: self.nonce.clone(),
        };
        let work = H256::from_slice(Sha3_256::digest(&calculation.encode()[..]).as_slice());

        Seal {
            nonce: self.nonce.clone(),
            difficulty: self.difficulty,
            work: H256::from(work),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sha3Algorithm;

impl<B: BlockT<Hash=H256>> PowAlgorithm<B> for Sha3Algorithm {
    type Difficulty = Difficulty;

    fn difficulty(&self, _parent: B::Hash) -> Result<Difficulty, Error<B>> {
        Ok(U256::from(10000))
    }

    fn verify(
        &self,
        _parent: &BlockId<B>,
        pre_hash: &H256,
        seal: &RawSeal,
        difficulty: Difficulty,
    ) -> Result<bool, Error<B>> {
        let seal = match Seal::decode(&mut &seal[..]) {
            Ok(seal) => seal,
            Err(_) => return Ok(false),
        };

        if !is_valid_hash(&seal.work, difficulty) {
            return Ok(false)
        }

        let compute = Compute {
            difficulty,
            pre_hash: *pre_hash,
            nonce: seal.nonce,
        };

        if compute.compute() != seal {
            return Ok(false)
        }

        Ok(true)
    }

    fn mine(
        &self,
        parent: &BlockId<B>,
        pre_hash: &H256,
        difficulty: Difficulty,
        round: u32,
    ) -> Result<Option<RawSeal>, Error<B>> {
        let mut rng = SmallRng::from_rng(&mut thread_rng())
            .map_err(|e| Error::Environment(format!("Initialize RNG failed for mining: {:?}", e)))?;

        for _ in 0..round {
            std::thread::sleep(Duration::new(1, 0));

            let nonce = H256::random_using(&mut rng);

            let compute = Compute {
                difficulty,
                pre_hash: *pre_hash,
                nonce,
            };

            let seal = compute.compute();

            if is_valid_hash(&seal.work, difficulty) {
                return Ok(Some(seal.encode()))
            }
        }

        Ok(None)
    }
}