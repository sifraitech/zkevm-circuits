//! Module which contains all the RPC calls that are needed at any point to
//! query a Geth node in order to get a Block, Tx or Trace info.

use crate::eth_types::{
    Address, Block, Bytes, GethExecTrace, Hash, ResultGethExecTraces,
    Transaction, Word, H256, U256, U64,
};
use crate::Error;
use ethers_providers::JsonRpcClient;
use serde::{Deserialize, Serialize, Serializer};

/// Serialize a type.
///
/// # Panics
///
/// If the type returns an error during serialization.
pub fn serialize<T: serde::Serialize>(t: &T) -> serde_json::Value {
    serde_json::to_value(t).expect("Types never fail to serialize.")
}

/// Struct used to define the input that you want to provide to the
/// `eth_getBlockByNumber` call as it mixes numbers with string literals.
#[derive(Debug)]
pub enum BlockNumber {
    /// Specific block number
    Num(U64),
    /// Earliest block
    Earliest,
    /// Latest block
    Latest,
    /// Pending block
    Pending,
}

impl From<u64> for BlockNumber {
    fn from(num: u64) -> Self {
        BlockNumber::Num(U64::from(num))
    }
}

impl Serialize for BlockNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            BlockNumber::Num(num) => num.serialize(serializer),
            BlockNumber::Earliest => "earliest".serialize(serializer),
            BlockNumber::Latest => "latest".serialize(serializer),
            BlockNumber::Pending => "pending".serialize(serializer),
        }
    }
}

/// Struct used to define the storage proof
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct StorageProof {
    /// Storage key
    pub key: H256,
    /// Storage proof: rlp-encoded trie nodes from root to value.
    pub proof: Vec<Bytes>,
    /// Storage Value
    pub value: U256,
}

/// Struct used to define the result of `eth_getProof` call
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct EIP1186ProofResponse {
    /// The balance of the account
    pub balance: U256,
    /// The hash of the code of the account
    pub codeHash: H256,
    /// The nonce of the account
    pub nonce: U256,
    /// SHA3 of the StorageRoot
    pub storageHash: H256,
    /// Array of rlp-serialized MerkleTree-Nodes
    pub accountProof: Vec<Bytes>,
    /// Array of storage-entries as requested
    pub storageProof: Vec<StorageProof>,
}

/// Placeholder structure designed to contain the methods that the BusMapping
/// needs in order to enable Geth queries.
pub struct GethClient<P: JsonRpcClient>(P);

impl<P: JsonRpcClient> GethClient<P> {
    /// Generates a new `GethClient` instance.
    pub fn new(provider: P) -> Self {
        Self(provider)
    }

    /// Calls `eth_getBlockByHash` via JSON-RPC returning a [`Block`] returning
    /// all the block information including it's transaction's details.
    pub async fn get_block_by_hash(
        &self,
        hash: Hash,
    ) -> Result<Block<Transaction>, Error> {
        let hash = serialize(&hash);
        let flag = serialize(&true);
        self.0
            .request("eth_getBlockByHash", [hash, flag])
            .await
            .map_err(|e| Error::JSONRpcError(e.into()))
    }

    /// Calls `eth_getBlockByNumber` via JSON-RPC returning a [`Block`]
    /// returning all the block information including it's transaction's
    /// details.
    pub async fn get_block_by_number(
        &self,
        block_num: BlockNumber,
    ) -> Result<Block<Transaction>, Error> {
        let num = serialize(&block_num);
        let flag = serialize(&true);
        self.0
            .request("eth_getBlockByNumber", [num, flag])
            .await
            .map_err(|e| Error::JSONRpcError(e.into()))
    }

    /// Calls `debug_traceBlockByHash` via JSON-RPC returning a
    /// [`Vec<GethExecTrace>`] with each GethTrace corresponding to 1
    /// transaction of the block.
    pub async fn trace_block_by_hash(
        &self,
        hash: Hash,
    ) -> Result<Vec<GethExecTrace>, Error> {
        let hash = serialize(&hash);
        let resp: ResultGethExecTraces = self
            .0
            .request("debug_traceBlockByHash", [hash])
            .await
            .map_err(|e| Error::JSONRpcError(e.into()))?;
        Ok(resp.0.into_iter().map(|step| step.result).collect())
    }

    /// Calls `debug_traceBlockByNumber` via JSON-RPC returning a
    /// [`Vec<GethExecTrace>`] with each GethTrace corresponding to 1
    /// transaction of the block.
    pub async fn trace_block_by_number(
        &self,
        block_num: BlockNumber,
    ) -> Result<Vec<GethExecTrace>, Error> {
        let num = serialize(&block_num);
        let resp: ResultGethExecTraces = self
            .0
            .request("debug_traceBlockByNumber", [num])
            .await
            .map_err(|e| Error::JSONRpcError(e.into()))?;
        Ok(resp.0.into_iter().map(|step| step.result).collect())
    }

    /// Calls `eth_getCode` via JSON-RPC returning a contract code
    pub async fn get_code_by_address(
        &self,
        contract_address: Address,
        block_num: BlockNumber,
    ) -> Result<Vec<u8>, Error> {
        let address = serialize(&contract_address);
        let num = serialize(&block_num);
        let resp: Bytes = self
            .0
            .request("eth_getCode", [address, num])
            .await
            .map_err(|e| Error::JSONRpcError(e.into()))?;
        Ok(resp.to_vec())
    }

    /// Calls `eth_getProof` via JSON-RPC returning a
    /// [`EIP1186ProofResponse`] returning the account and
    /// storage-values of the specified account including the Merkle-proof.
    pub async fn get_proof(
        &self,
        account: Address,
        keys: Vec<Word>,
        block_num: BlockNumber,
    ) -> Result<EIP1186ProofResponse, Error> {
        let account = serialize(&account);
        let keys = serialize(&keys);
        let num = serialize(&block_num);
        self.0
            .request("eth_getProof", [account, keys, num])
            .await
            .map_err(|e| Error::JSONRpcError(e.into()))
    }
}

// Integration tests found in `integration-tests/tests/rpc.rs`.
