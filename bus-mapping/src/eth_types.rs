//! TODO

use crate::evm::{Gas, GasCost, OpcodeId, ProgramCounter};
use pasta_curves::arithmetic::FieldExt;
use serde::{de, Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use web3::types;

use subtle::CtOption;
// use ethereum_types::{H160, H2048, H256, H64, U256, U64};

/*
/// TODO
pub trait ToField {
    /// TODO
    type Field: FieldExt;
    /// TODO
    fn to_field(&self) -> Self::Field;
}
*/

/// TODO
pub trait ToField<F: FieldExt> {
    /// TODO
    fn to_field(&self) -> CtOption<F>;
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

impl<F: FieldExt> ToField<F> for U256 {
    fn to_field(&self) -> CtOption<F> {
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

impl<F: FieldExt> ToField<F> for Address {
    fn to_field(&self) -> CtOption<F> {
        let mut bytes = [0u8; 32];
        bytes[32 - Self::len_bytes()..].copy_from_slice(self.as_bytes());
        F::from_bytes(&bytes)
    }
}

// From `web3/types/block.rs`
/// The block type returned from RPC calls.
/// This is generic over a `TX` type.
pub type Block = types::Block<Transaction>;
//#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
//pub struct Block<TX> {
//    /// Hash of the block
//    pub hash: Option<H256>,
//    /// Hash of the parent
//    #[serde(rename = "parentHash")]
//    pub parent_hash: H256,
//    /// Hash of the uncles
//    #[serde(rename = "sha3Uncles")]
//    pub uncles_hash: H256,
//    /// Miner/author's address.
//    #[serde(rename = "miner", default, deserialize_with = "null_to_default")]
//    pub author: H160,
//    /// State root hash
//    #[serde(rename = "stateRoot")]
//    pub state_root: H256,
//    /// Transactions root hash
//    #[serde(rename = "transactionsRoot")]
//    pub transactions_root: H256,
//    /// Transactions receipts root hash
//    #[serde(rename = "receiptsRoot")]
//    pub receipts_root: H256,
//    /// Block number. None if pending.
//    pub number: Option<U64>,
//    /// Gas Used
//    #[serde(rename = "gasUsed")]
//    pub gas_used: U256,
//    /// Gas Limit
//    #[serde(rename = "gasLimit")]
//    pub gas_limit: U256,
//    /// Base fee per unit of gas (if past London)
//    #[serde(rename = "baseFeePerGas")]
//    pub base_fee_per_gas: Option<U256>,
//    /// Extra data
//    #[serde(rename = "extraData")]
//    pub extra_data: Bytes,
//    /// Logs bloom
//    #[serde(rename = "logsBloom")]
//    pub logs_bloom: Option<H2048>,
//    /// Timestamp
//    pub timestamp: U256,
//    /// Difficulty
//    pub difficulty: U256,
//    /// Total difficulty
//    #[serde(rename = "totalDifficulty")]
//    pub total_difficulty: Option<U256>,
//    /// Seal fields
//    #[serde(default, rename = "sealFields")]
//    pub seal_fields: Vec<Bytes>,
//    /// Uncles' hashes
//    pub uncles: Vec<H256>,
//    /// Transactions
//    pub transactions: Vec<TX>,
//    /// Size in bytes
//    pub size: Option<U256>,
//    /// Mix Hash
//    #[serde(rename = "mixHash")]
//    pub mix_hash: Option<H256>,
//    /// Nonce
//    pub nonce: Option<H64>,
//}

/// TODO
pub type Transaction = types::Transaction;

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
    pub stack: Vec<Word>,
    // memory is in chunks of 32 bytes, in hex
    #[serde(default)]
    pub memory: Vec<Word>,
    // storage is hex -> hex
    #[serde(default)]
    pub storage: HashMap<Word, Word>,
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

#[cfg(test)]
macro_rules! address {
    ($addr_hex:expr) => {
        Address::from_str(&$addr_hex).expect("invalid hex Address")
    };
}

#[cfg(test)]
macro_rules! word {
    ($word_hex:expr) => {
        Word::from_str_radix(&$word_hex, 16).expect("invalid hex Word")
    };
}

#[cfg(test)]
macro_rules! word_map {
    () => {
        HashMap::new()
    };
    ($($key_hex:expr => $value_hex:expr),*) => {
        {
            use std::iter::FromIterator;
            HashMap::from_iter([(
                    $(word!($key_hex), word!($value_hex)),*
            )])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evm::opcodes::ids::OpcodeId;

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
                        stack: vec![],
                        storage: word_map!(),
                        memory: vec![],
                    },
                    GethExecStep {
                        pc: ProgramCounter(163),
                        op: OpcodeId::SLOAD,
                        gas: Gas(5217),
                        gas_cost: GasCost(2100),
                        depth: 1,
                        stack: vec![
                            word!("0x1003e2d2"),
                            word!("0x2a"),
                            word!("0x0")
                        ],
                        storage: word_map!("0x0" => "0x6f"),
                        memory: vec![
                            word!("0x0"),
                            word!("0x0"),
                            word!("0x080")
                        ],
                    }
                ],
            }
        );
    }
}

#[cfg(test)]
mod eth_types_test {
    use super::*;
    use crate::Error;

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

        // TODO
        /*
        // Test to_word
        assert_eq!(
            Address::from_str("0x0000000000000000000000000000000000000001")
                .unwrap()
                .to_word(),
            EvmWord::from(1u32),
        )
        */
    }
}
