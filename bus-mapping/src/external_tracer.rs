//! This module generates traces by connecting to an external tracer
use crate::eth_types::{Address, Word};
use crate::Error;
use crate::{
    bytecode::Bytecode, BlockConstants, ExecutionStep, ExecutionTrace,
};
use geth_utils;
use pasta_curves::arithmetic::FieldExt;
use serde::Serialize;
use std::str::FromStr;

/// Definition of all of the constants related to an Ethereum transaction.
#[derive(Debug, Clone, Serialize)]
pub struct Transaction {
    origin: Address,
    gas_limit: Word,
    target: Address,
}

impl Default for Transaction {
    fn default() -> Self {
        Transaction {
            origin: Address::from_str(
                "0x00000000000000000000000000000000c014ba5e",
            )
            .unwrap(),
            gas_limit: Word::from(1_000_000u64),
            target: Address::zero(),
        }
    }
}

/// Definition of all of the data related to an account.
#[derive(Debug, Clone, Serialize)]
pub struct Account {
    address: Address,
    balance: Word,
    code: String,
}

#[derive(Debug, Clone, Serialize)]
struct GethConfig {
    block_constants: BlockConstants,
    transaction: Transaction,
    accounts: Vec<Account>,
}

/// Creates a trace for the specified config
pub fn trace(
    block_constants: &BlockConstants,
    code: &Bytecode,
) -> Result<Vec<ExecutionStep>, Error> {
    // Some default values for now
    let transaction = Transaction::default();
    let account = Account {
        address: transaction.target,
        balance: Word::from(555u64),
        code: hex::encode(code.to_bytes()),
    };

    let geth_config = GethConfig {
        block_constants: block_constants.clone(),
        transaction,
        accounts: vec![account],
    };

    // Get the trace
    let trace =
        geth_utils::trace(&serde_json::to_string(&geth_config).unwrap())
            .map_err(|_| Error::TracingError)?;

    // Generate the execution steps
    ExecutionTrace::load_trace(trace.as_bytes())
}
