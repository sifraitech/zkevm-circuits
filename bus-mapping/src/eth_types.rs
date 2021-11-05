//! TODO

use crate::evm::{memory::Memory, stack::Stack, storage::Storage};
use crate::evm::{Gas, GasCost, OpcodeId, ProgramCounter};
use pasta_curves::arithmetic::FieldExt;
use serde::{de, Deserialize, Deserializer, Serialize};
// use std::collections::HashMap;
use std::str::FromStr;
pub use web3::types::{self, AccessList, Bytes, Index, H2048, H64, U64};

use subtle::CtOption;
// use ethereum_types::{H160, H2048, H256, H64, U256, U64};

/// TODO
// TODO Rename to ToScalar
pub trait ToScalar<F: FieldExt> {
    /// TODO
    fn to_scalar(&self) -> CtOption<F>;
}

uint::construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

// impl_serde::impl_uint_serde!(U256, 4);

impl<'de> Deserialize<'de> for U256 {
    fn deserialize<D>(deserializer: D) -> Result<U256, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        U256::from_str(&s).map_err(de::Error::custom)
    }
}

impl Serialize for U256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = format!("{:#x}", self);
        if s == "0x00" {
            serializer.serialize_str("0x0")
        } else {
            serializer.serialize_str(&s)
        }
    }
}

impl<F: FieldExt> ToScalar<F> for U256 {
    fn to_scalar(&self) -> CtOption<F> {
        let mut bytes = [0u8; 32];
        self.to_little_endian(&mut bytes);
        F::from_bytes(&bytes)
    }
}

impl U256 {
    /// TODO
    pub fn to_be_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        self.to_big_endian(&mut bytes);
        bytes
    }
}

/// TODO
// pub type Word = types::U256;
pub type Word = U256;

/// TODO
pub type Hash = types::H256;

/// TODO
pub trait ToWord {
    /// TODO
    fn to_word(&self) -> Word;
}

/// TODO
// pub type Address = types::Address;
pub use types::Address;
// pub type Address = H160;

impl ToWord for Address {
    fn to_word(&self) -> Word {
        let mut bytes = [0u8; 32];
        bytes[32 - Self::len_bytes()..].copy_from_slice(self.as_bytes());
        Word::from(bytes)
    }
}

impl<F: FieldExt> ToScalar<F> for Address {
    fn to_scalar(&self) -> CtOption<F> {
        let mut bytes = [0u8; 32];
        bytes[32 - Self::len_bytes()..].copy_from_slice(self.as_bytes());
        F::from_bytes(&bytes)
    }
}

fn null_to_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let option = Option::deserialize(deserializer)?;
    Ok(option.unwrap_or_default())
}

// pub type Block = types::Block<Transaction>;

// From `web3/types/block.rs`
/// The block type returned from RPC calls.
/// This is generic over a `TX` type.
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct Block<TX> {
    /// Hash of the block
    pub hash: Option<Hash>,
    /// Hash of the parent
    #[serde(rename = "parentHash")]
    pub parent_hash: Hash,
    /// Hash of the uncles
    #[serde(rename = "sha3Uncles")]
    pub uncles_hash: Hash,
    /// Miner/author's address.
    #[serde(rename = "miner", default, deserialize_with = "null_to_default")]
    pub author: Address,
    /// State root hash
    #[serde(rename = "stateRoot")]
    pub state_root: Hash,
    /// Transactions root hash
    #[serde(rename = "transactionsRoot")]
    pub transactions_root: Hash,
    /// Transactions receipts root hash
    #[serde(rename = "receiptsRoot")]
    pub receipts_root: Hash,
    /// Block number. None if pending.
    pub number: Option<U64>,
    /// Gas Used
    #[serde(rename = "gasUsed")]
    pub gas_used: Word,
    /// Gas Limit
    #[serde(rename = "gasLimit")]
    pub gas_limit: Word,
    /// Base fee per unit of gas (if past London)
    #[serde(rename = "baseFeePerGas")]
    pub base_fee_per_gas: Option<Word>,
    /// Extra data
    #[serde(rename = "extraData")]
    pub extra_data: Bytes,
    /// Logs bloom
    #[serde(rename = "logsBloom")]
    pub logs_bloom: Option<H2048>,
    /// Timestamp
    pub timestamp: Word,
    /// Difficulty
    pub difficulty: Word,
    /// Total difficulty
    #[serde(rename = "totalDifficulty")]
    pub total_difficulty: Option<Word>,
    /// Seal fields
    #[serde(default, rename = "sealFields")]
    pub seal_fields: Vec<Bytes>,
    /// Uncles' hashes
    pub uncles: Vec<Hash>,
    /// Transactions
    pub transactions: Vec<TX>,
    /// Size in bytes
    pub size: Option<Word>,
    /// Mix Hash
    #[serde(rename = "mixHash")]
    pub mix_hash: Option<Hash>,
    /// Nonce
    pub nonce: Option<H64>,
}

impl Block<()> {
    /// TODO
    pub fn mock() -> Self {
        Self {
            hash: Some(Hash::from([0u8; 32])),
            parent_hash: Hash::from([0u8; 32]),
            uncles_hash: Hash::from([0u8; 32]),
            author: Address::from([0u8; 20]),
            state_root: Hash::from([0u8; 32]),
            transactions_root: Hash::from([0u8; 32]),
            receipts_root: Hash::from([0u8; 32]),
            number: Some(U64([123456u64])),
            gas_used: Word::from(15_000_000u64),
            gas_limit: Word::from(15_000_000u64),
            base_fee_per_gas: Some(Word::from(97u64)),
            extra_data: Bytes(Vec::new()),
            logs_bloom: None,
            timestamp: Word::from(1633398551u64),
            difficulty: Word::from(0x200000u64),
            total_difficulty: None,
            seal_fields: Vec::new(),
            uncles: Vec::new(),
            transactions: Vec::new(),
            size: None,
            mix_hash: None,
            nonce: Some(H64([0u8; 8])),
        }
    }
}

// /// TODO
// pub type Transaction = types::Transaction;

// From `web3/types/transaction.rs`
/// The Transaction type returned from RPC calls.
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct Transaction {
    /// Hash
    pub hash: Hash,
    /// Nonce
    pub nonce: Word,
    /// Block hash. None when pending.
    #[serde(rename = "blockHash")]
    pub block_hash: Option<Hash>,
    /// Block number. None when pending.
    #[serde(rename = "blockNumber")]
    pub block_number: Option<U64>,
    /// Transaction Index. None when pending.
    #[serde(rename = "transactionIndex")]
    pub transaction_index: Option<Index>,
    /// Sender
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Address>,
    /// Recipient (None when contract creation)
    pub to: Option<Address>,
    /// Transfered value
    pub value: Word,
    /// Gas Price
    #[serde(rename = "gasPrice")]
    pub gas_price: Word,
    /// Gas amount
    pub gas: Word,
    /// Input data
    pub input: Bytes,
    /// ECDSA recovery id
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v: Option<U64>,
    /// ECDSA signature r, 32 bytes
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r: Option<Word>,
    /// ECDSA signature s, 32 bytes
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s: Option<Word>,
    /// Raw transaction data
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<Bytes>,
    /// Transaction type, Some(1) for AccessList transaction, None for Legacy
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub transaction_type: Option<U64>,
    /// Access list
    #[serde(
        rename = "accessList",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub access_list: Option<AccessList>,
}

impl Transaction {
    /// TODO
    pub fn mock<TX>(block: &Block<TX>) -> Self {
        Self {
            hash: Hash::from([0u8; 32]),
            nonce: Word::from([0u8; 32]),
            block_hash: block.hash,
            block_number: block.number,
            transaction_index: Some(Index::from(0u64)),
            from: Some(
                Address::from_str("0x00000000000000000000000000000000c014ba5e")
                    .unwrap(),
            ),
            to: Some(Address::zero()),
            value: Word::from([0u8; 32]),
            gas_price: Word::from([0u8; 32]),
            gas: Word::from(1_000_000u64),
            input: Bytes(Vec::new()),
            v: Some(U64([0u64])),
            r: Some(Word::from([0u8; 32])),
            s: Some(Word::from([0u8; 32])),
            raw: Some(Bytes(Vec::new())),
            transaction_type: Some(U64([0u64])),
            access_list: None,
        }
    }
}

/// TODO Corresponds to `StructLogRes` in `go-ethereum/internal/ethapi/api.go`.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[doc(hidden)]
pub struct GethExecStep {
    pub pc: ProgramCounter,
    pub op: OpcodeId,
    pub gas: Gas,
    #[serde(alias = "gasCost")]
    pub gas_cost: GasCost,
    pub depth: u8,
    // pub(crate) error: &'a str,
    // stack is in hex 0x prefixed
    // pub stack: Vec<Word>,
    pub stack: Stack,
    // memory is in chunks of 32 bytes, in hex
    // pub memory: Vec<Word>,
    #[serde(default)]
    pub memory: Memory,
    // storage is hex -> hex
    #[serde(default)]
    // pub storage: HashMap<Word, Word>,
    pub storage: Storage,
}

/// TODO Corresponds to `ExecutionResult` in `go-ethereum/internal/ethapi/api.go`
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[doc(hidden)]
pub struct GethExecTrace {
    pub gas: Gas,
    pub failed: bool,
    // return_value is a hex encoded byte array
    // #[serde(alias = "returnValue")]
    // pub(crate) return_value: String,
    #[serde(alias = "structLogs")]
    pub struct_logs: Vec<GethExecStep>,
}

// TODO: Move this test macros to a crate, export them, and use them in all tests

#[macro_export]
/// TODO: Panics
macro_rules! address {
    ($addr_hex:expr) => {{
        use std::str::FromStr;
        $crate::eth_types::Address::from_str(&$addr_hex)
            .expect("invalid hex Address")
    }};
}

#[macro_export]
/// TODO: Panics
macro_rules! word {
    ($word_hex:expr) => {
        $crate::eth_types::Word::from_str_radix(&$word_hex, 16)
            .expect("invalid hex Word")
    };
}

#[macro_export]
/// TODO: Panics
macro_rules! word_map {
    () => {
        std::collections::HashMap::new()
    };
    ($($key_hex:expr => $value_hex:expr),*) => {
        {
            use std::iter::FromIterator;
            std::collections::HashMap::from_iter([(
                    $(word!($key_hex), word!($value_hex)),*
            )])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evm::opcodes::ids::OpcodeId;
    use crate::evm::{memory::Memory, stack::Stack};

    #[test]
    fn deserialize_geth_exec_trace2() {
        let trace_json = r#"
  {
    "gas": 26809,
    "failed": false,
    "returnValue": "",
    "structLogs": [
      {
        "pc": 0,
        "op": "PUSH1",
        "gas": 22705,
        "gasCost": 3,
        "depth": 1,
        "stack": []
      },
      {
        "pc": 163,
        "op": "SLOAD",
        "gas": 5217,
        "gasCost": 2100,
        "depth": 1,
        "stack": [
          "0x1003e2d2",
          "0x2a",
          "0x0"
        ],
        "storage": {
          "0000000000000000000000000000000000000000000000000000000000000000": "000000000000000000000000000000000000000000000000000000000000006f"
        },
        "memory": [
          "0000000000000000000000000000000000000000000000000000000000000000",
          "0000000000000000000000000000000000000000000000000000000000000000",
          "0000000000000000000000000000000000000000000000000000000000000080"
        ]
      }
    ]
  }
        "#;
        let trace: GethExecTrace = serde_json::from_str(trace_json)
            .expect("json-deserialize GethExecTrace");
        assert_eq!(
            trace,
            GethExecTrace {
                gas: Gas(26809),
                failed: false,
                struct_logs: vec![
                    GethExecStep {
                        pc: ProgramCounter(0),
                        op: OpcodeId::PUSH1,
                        gas: Gas(22705),
                        gas_cost: GasCost(3),
                        depth: 1,
                        stack: Stack::new(),
                        storage: Storage(word_map!()),
                        memory: Memory::new(),
                    },
                    GethExecStep {
                        pc: ProgramCounter(163),
                        op: OpcodeId::SLOAD,
                        gas: Gas(5217),
                        gas_cost: GasCost(2100),
                        depth: 1,
                        stack: Stack(vec![
                            word!("0x1003e2d2"),
                            word!("0x2a"),
                            word!("0x0")
                        ]),
                        storage: Storage(word_map!("0x0" => "0x6f")),
                        memory: Memory::from(vec![
                            word!("0x0"),
                            word!("0x0"),
                            word!("0x080")
                        ]),
                    }
                ],
            }
        );
    }
}

#[cfg(test)]
mod eth_types_test {
    use super::*;

    #[test]
    fn address() {
        // Test from_str
        assert_eq!(
            Address::from_str("0x9a0C63EBb78B35D7c209aFbD299B056098b5439b")
                .unwrap(),
            Address::from([
                154, 12, 99, 235, 183, 139, 53, 215, 194, 9, 175, 189, 41, 155,
                5, 96, 152, 181, 67, 155
            ])
        );
        assert_eq!(
            Address::from_str("9a0C63EBb78B35D7c209aFbD299B056098b5439b")
                .unwrap(),
            Address::from([
                154, 12, 99, 235, 183, 139, 53, 215, 194, 9, 175, 189, 41, 155,
                5, 96, 152, 181, 67, 155
            ])
        );

        // Test from_str Errors
        assert_eq!(
            &format!(
                "{:?}",
                Address::from_str("0x9a0C63EBb78B35D7c209aFbD299B056098b543")
            ),
            "Err(Invalid input length)",
        );
        assert_eq!(
            &format!(
                "{:?}",
                Address::from_str("0x9a0C63EBb78B35D7c209aFbD299B056098b543XY")
            ),
            "Err(Invalid character 'X' at position 38)",
        );

        // Test to_word
        assert_eq!(
            Address::from_str("0x0000000000000000000000000000000000000001")
                .unwrap()
                .to_word(),
            Word::from(1u32),
        )
    }
}
