//! This module generates traces by connecting to an external tracer
use crate::eth_types::{self, Address, GethExecStep, Word};
use crate::BlockConstants;
use crate::Error;
use geth_utils;
use serde::Serialize;

/// Definition of all of the constants related to an Ethereum transaction.
#[derive(Debug, Clone, Serialize)]
pub struct Transaction {
    /// Origin Address
    pub origin: Address,
    /// Gas Limit
    pub gas_limit: Word,
    /// Target Address
    pub target: Address,
}

impl Transaction {
    /// Create Self from a web3 transaction
    pub fn from_eth_tx(tx: &eth_types::Transaction) -> Self {
        Self {
            origin: tx.from.unwrap(),
            gas_limit: tx.gas,
            target: tx.to.unwrap(),
        }
    }
}

/// Definition of all of the data related to an account.
#[derive(Debug, Clone, Serialize)]
pub struct Account {
    /// Address
    pub address: Address,
    /// Balance
    pub balance: Word,
    /// EVM Code
    pub code: String,
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
) -> Result<Vec<GethExecStep>, Error> {
    let geth_config = GethConfig {
        block_constants: block_constants.clone(),
        transaction: tx.clone(),
        accounts: accounts.to_vec(),
    };

    // Get the trace
    let trace_string =
        geth_utils::trace(&serde_json::to_string(&geth_config).unwrap())
            .map_err(|_| Error::TracingError)?;

    let trace: Vec<GethExecStep> =
        serde_json::from_str(&trace_string).map_err(Error::SerdeError)?;
    Ok(trace)
}

/// TODO
pub const ERR_STACK_OVERFLOW: &str = "stack limit reached";
/// TODO
pub const ERR_INVALID_OPCODE: &str = "invalid opcode";
/// TODO
pub const ERR_STACK_UNDERFLOW: &str = "stack underflow";
/// TODO
pub const ERR_OUT_OF_GAS: &str = "out of gas";
/// TODO
pub const ERR_WRITE_PROTECTION: &str = "write protection";

#[cfg(test)]
mod tracer_tests {
    use super::*;
    use crate::{
        address, bytecode,
        evm::{stack::Stack, OpcodeId},
        mock, word,
    };

    #[test]
    fn tracer_err_stack_overflow() {
        let mut code = bytecode::Bytecode::default();
        for i in 0..1025 {
            code.push(2, Word::from(i));
        }
        let block = mock::BlockData::new_single_tx_trace_code(&code).unwrap();
        let last_step = &block.geth_trace.struct_logs
            [block.geth_trace.struct_logs.len() - 1];
        assert_eq!(
            last_step.error,
            Some(format!("{} 1024 (1023)", ERR_STACK_OVERFLOW))
        );
    }

    #[test]
    fn tracer_err_stack_underflow() {
        let code = bytecode! {
            SWAP5
        };
        let block = mock::BlockData::new_single_tx_trace_code(&code).unwrap();
        assert_eq!(
            block.geth_trace.struct_logs[0].error,
            Some(format!("{} (0 <=> 6)", ERR_STACK_UNDERFLOW))
        );
    }

    #[test]
    fn tracer_err_out_of_gas() {
        let code = bytecode! {
            PUSH1(0x0)
            PUSH1(0x1)
            PUSH1(0x2)
        };

        let eth_block = mock::new_block();
        let mut eth_tx = mock::new_tx(&eth_block);
        eth_tx.gas = Word::from(4);
        let block_ctants = BlockConstants::from_eth_block(
            &eth_block,
            &eth_types::Word::one(),
            &address!("0x00000000000000000000000000000000c014ba5e"),
        );
        let tracer_tx = Transaction::from_eth_tx(&eth_tx);
        let tracer_account = mock::new_tracer_account(&code);
        let struct_logs = trace(&block_ctants, &tracer_tx, &[tracer_account])
            .unwrap()
            .to_vec();

        assert_eq!(struct_logs[1].error, Some(ERR_OUT_OF_GAS.to_string()));
    }

    #[test]
    fn tracer_err_write_protection() {
        let code_a = bytecode! {
            PUSH1(0x0) // retLength
            PUSH1(0x0) // retOffset
            PUSH1(0x0) // argsLength
            PUSH1(0x0) // argsOffset
            PUSH32(word!("0x0000000000000000000000000000000000000123")) // addr
            PUSH32(0x10_000) // gas
            STATICCALL

            PUSH2(0xaa)
        };

        let code_b = bytecode! {
            PUSH1(0x01) // value
            PUSH1(0x02) // key
            SSTORE

            PUSH3(0xbb)
        };

        let eth_block = mock::new_block();
        let eth_tx = mock::new_tx(&eth_block);
        let block_ctants = BlockConstants::from_eth_block(
            &eth_block,
            &eth_types::Word::one(),
            &address!("0x00000000000000000000000000000000c014ba5e"),
        );
        let tracer_tx = Transaction::from_eth_tx(&eth_tx);
        let tracer_account_a = mock::new_tracer_account(&code_a);
        let mut tracer_account_b = mock::new_tracer_account(&code_b);
        tracer_account_b.address =
            address!("0x0000000000000000000000000000000000000123");
        let struct_logs = trace(
            &block_ctants,
            &tracer_tx,
            &[tracer_account_a, tracer_account_b],
        )
        .unwrap()
        .to_vec();
        assert_eq!(
            struct_logs[9].error,
            Some(ERR_WRITE_PROTECTION.to_string())
        );
    }

    // Depth error condition: `depth` = 1025, `op` in {CALL, CALLCODE, DELEGATECALL, STATICCALL}
    #[test]
    fn tracer_err_depth() {
        // Recursive CALL will exaust the call depth
        let code = bytecode! {
                 PUSH1(0x0) // retLength
                 PUSH1(0x0) // retOffset
                 PUSH1(0x0) // argsLength
                 PUSH1(0x0) // argsOffset
                 PUSH1(0x42) // value
                 PUSH32(word!("0x0000000000000000000000000000000000000000")) // addr
                 PUSH32(0x8_000_000_000_000u64) // gas
                 CALL
                 PUSH2(0xab)
                 STOP
        };

        let eth_block = mock::new_block();
        let mut eth_tx = mock::new_tx(&eth_block);
        eth_tx.gas = Word::from(1_000_000_000_000_000u64);
        let block_ctants = BlockConstants::from_eth_block(
            &eth_block,
            &eth_types::Word::one(),
            &address!("0x00000000000000000000000000000000c014ba5e"),
        );
        let tracer_tx = Transaction::from_eth_tx(&eth_tx);
        let tracer_account = mock::new_tracer_account(&code);
        let struct_logs = trace(&block_ctants, &tracer_tx, &[tracer_account])
            .unwrap()
            .to_vec();
        // get last CALL
        let (index, last_step) = struct_logs
            .iter()
            .enumerate()
            .rev()
            .find(|(_, s)| s.op == OpcodeId::CALL)
            .unwrap();
        assert_eq!(last_step.op, OpcodeId::CALL);
        assert_eq!(last_step.depth, 1025u16);
        // Unfortunately the trace doesn't record errors generated by a CALL.  We only get the
        // success = 0 the next step's stack
        assert_eq!(last_step.error, None);
        assert_eq!(struct_logs[index + 1].op, OpcodeId::PUSH2);
        assert_eq!(struct_logs[index + 1].depth, 1025u16);
        assert_eq!(struct_logs[index + 1].stack, Stack(vec![Word::from(0)])); // success = 0
        assert_eq!(struct_logs[index + 2].op, OpcodeId::STOP);
        assert_eq!(struct_logs[index + 2].depth, 1025u16);
    }

    // insufficient balance
    // TODO

    // contract address collision
    // TODO

    // invalid code
    // TODO.  Triggered on create

    #[test]
    fn tracer_err_invalid_jump() {
        let code = bytecode! {
            PUSH1(0x10)
            JUMP
            STOP
        };
        let block = mock::BlockData::new_single_tx_trace_code(&code).unwrap();
        assert_eq!(block.geth_trace.struct_logs.len(), 2);
        assert_eq!(block.geth_trace.struct_logs[1].op, OpcodeId::JUMP);
        // println!("{:#?}", block.geth_trace.struct_logs);
        // The error is not found in the GethExecStep. :(
    }

    #[test]
    fn tracer_err_execution_reverted() {
        let code = bytecode! {
            PUSH1(0x0)
            PUSH2(0x0)
            REVERT
            PUSH3(0x12)
            STOP
        };
        let block = mock::BlockData::new_single_tx_trace_code(&code).unwrap();
        assert_eq!(block.geth_trace.struct_logs.len(), 3);
        assert_eq!(block.geth_trace.struct_logs[2].op, OpcodeId::REVERT);
    }

    // return data out of bounds
    // TODO

    #[test]
    fn tracer_err_invalid_opcode() {
        let mut code = bytecode::Bytecode::default();
        code.write_op(OpcodeId::PC);
        code.write(0x0f);
        let block = mock::BlockData::new_single_tx_trace_code(&code).unwrap();
        let last_step = &block.geth_trace.struct_logs
            [block.geth_trace.struct_logs.len() - 1];
        // println!("{:#?}", last_step);
        assert_eq!(last_step.op, OpcodeId::INVALID(0x0f));
        assert_eq!(
            last_step.error,
            Some(format!("{}: opcode 0xf not defined", ERR_INVALID_OPCODE))
        );
    }
}
