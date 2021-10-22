//! This module generates traces by connecting to an external tracer
use crate::bytecode::Bytecode;
use crate::eth_types::{self, Address, GethExecStep, Word};
use crate::BlockConstants;
use crate::Error;
use geth_utils;
// use pasta_curves::arithmetic::FieldExt;
use serde::Serialize;

/// Definition of all of the constants related to an Ethereum transaction.
#[derive(Debug, Clone, Serialize)]
pub struct Transaction {
    origin: Address,
    gas_limit: Word,
    target: Address,
}

impl Transaction {
    /// TODO
    pub fn from_eth_tx(tx: &eth_types::Transaction) -> Self {
        Self {
            origin: tx.from.unwrap(),
            gas_limit: tx.gas,
            target: tx.to.unwrap(),
        }
    }

    /// TODO
    pub fn mock() -> Self {
        Transaction {
            origin: crate::address!(
                "0x00000000000000000000000000000000c014ba5e"
            ),
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

impl Account {
    /// TODO
    pub fn mock(code: &Bytecode) -> Self {
        Self {
            address: Transaction::mock().target,
            balance: Word::from(555u64),
            code: hex::encode(code.to_bytes()),
        }
    }
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
    tx: &Transaction,
    accounts: &[Account],
    // code: &Bytecode,
) -> Result<Vec<GethExecStep>, Error> {
    // Some default values for now
    // let transaction = Transaction::default();
    // let account = Account {
    //     address: transaction.target,
    //     balance: Word::from(555u64),
    //     code: hex::encode(code.to_bytes()),
    // };

    let geth_config = GethConfig {
        block_constants: block_constants.clone(),
        transaction: tx.clone(),
        accounts: accounts.to_vec(),
    };

    // Get the trace
    let trace_string =
        geth_utils::trace(&serde_json::to_string(&geth_config).unwrap())
            // TODO: Capture error into TracingError if possible
            .map_err(|_| Error::TracingError)?;

    // TODO: Capture error into TracingError if possible
    let trace: Vec<GethExecStep> =
        serde_json::from_str(&trace_string).map_err(|_| Error::TracingError)?;
    Ok(trace)
    // Generate the execution steps
    // ExecutionTrace::load_trace(trace.as_bytes())
}
