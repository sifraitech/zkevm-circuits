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

    // println!("DBG2 {}", serde_json::to_string(&geth_config).unwrap());
    // Get the trace
    let trace_string =
        geth_utils::trace(&serde_json::to_string(&geth_config).unwrap())
            .map_err(|_| Error::TracingError)?;

    let trace: Vec<GethExecStep> =
        serde_json::from_str(&trace_string).map_err(Error::SerdeError)?;
    Ok(trace)
}

/// Geth error message for stack overflow
pub const ERR_STACK_OVERFLOW: &str = "stack limit reached";
/// Geth error message for invalid opcode
pub const ERR_INVALID_OPCODE: &str = "invalid opcode";
/// Geth error message for stack underflow
pub const ERR_STACK_UNDERFLOW: &str = "stack underflow";
/// Geth error message for out of gas
pub const ERR_OUT_OF_GAS: &str = "out of gas";
/// Geth error message for gas uint64 overflow
pub const ERR_GAS_UINT_OVERFLOW: &str = "gas uint64 overflow";
/// Geth error message for write protection
pub const ERR_WRITE_PROTECTION: &str = "write protection";

#[cfg(test)]
mod tracer_tests {
    use super::*;
    use crate::{
        address, bytecode,
        bytecode::Bytecode,
        evm::{stack::Stack, OpcodeId},
        mock, word,
    };

    //
    // Useful test functions
    //

    // Generate a trace with code_a and code_b, where code_b is in address 0x123
    fn trace_code_2(code_a: &Bytecode, code_b: &Bytecode) -> Vec<GethExecStep> {
        let eth_block = mock::new_block();
        let eth_tx = mock::new_tx(&eth_block);
        let block_ctants = BlockConstants::from_eth_block(
            &eth_block,
            &eth_types::Word::one(),
            &address!("0x00000000000000000000000000000000c014ba5e"),
        );
        let tracer_tx = Transaction::from_eth_tx(&eth_tx);
        let tracer_account_a = mock::new_tracer_account(code_a);
        let mut tracer_account_b = mock::new_tracer_account(code_b);
        tracer_account_b.address =
            address!("0x0000000000000000000000000000000000000123");
        trace(
            &block_ctants,
            &tracer_tx,
            &[tracer_account_a, tracer_account_b],
        )
        .unwrap()
        .to_vec()
    }

    //
    // Errors ignored
    //

    fn check_err_depth(
        step: &GethExecStep,
        next_step: Option<&GethExecStep>,
    ) -> bool {
        [
            OpcodeId::CALL,
            OpcodeId::CALLCODE,
            OpcodeId::DELEGATECALL,
            OpcodeId::STATICCALL,
            OpcodeId::CREATE,
            OpcodeId::CREATE2,
        ]
        .contains(&step.op)
            && step.error.is_none()
            && result(next_step) == Word::zero()
            && step.depth == 1025
    }

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

        // Check
        assert_eq!(
            check_err_depth(&last_step, struct_logs.get(index + 1)),
            true
        );
    }

    // TODO
    fn check_err_insufficient_balance(
        _step: &GethExecStep,
        _next_step: Option<&GethExecStep>,
    ) -> bool {
        unimplemented!()
    }

    #[test]
    fn tracer_err_insufficient_balance() {
        let code_a = bytecode! {
            PUSH1(0x0) // retLength
            PUSH1(0x0) // retOffset
            PUSH1(0x0) // argsLength
            PUSH1(0x0) // argsOffset
            PUSH32(Word::from(0x1_000)) // value
            PUSH32(word!("0x0000000000000000000000000000000000000123")) // addr
            PUSH32(0x10_000) // gas
            CALL

            PUSH2(0xaa)
        };
        let code_b = bytecode! {
            PUSH1(0x01) // value
            PUSH1(0x02) // key
            SSTORE

            PUSH3(0xbb)
        };
        let struct_logs = trace_code_2(&code_a, &code_b);

        // get last CALL
        let (index, last_step) = struct_logs
            .iter()
            .enumerate()
            .rev()
            .find(|(_, s)| s.op == OpcodeId::CALL)
            .unwrap();
        // println!("{:#?}", &struct_logs[index - 1..index + 3]);
        // Unfortunately the trace doesn't record errors generated by a CALL.  We only get the
        // success = 0 the next step's stack
        assert_eq!(last_step.error, None);
        assert_eq!(struct_logs[index + 1].op, OpcodeId::PUSH2);
        assert_eq!(struct_logs[index + 1].stack, Stack(vec![Word::from(0)])); // success = 0
    }

    // TODO
    fn check_err_address_collision(
        _step: &GethExecStep,
        _next_step: Option<&GethExecStep>,
    ) -> bool {
        unimplemented!()
    }

    // contract address collision
    // TODO.  Triggered on create
    #[test]
    fn tracer_err_address_collision() {
        unimplemented!()
    }

    // TODO
    fn check_err_code_size_exceeded(
        _step: &GethExecStep,
        _next_step: Option<&GethExecStep>,
    ) -> bool {
        unimplemented!()
    }

    // contract address collision
    // TODO.  Triggered on create
    #[test]
    fn tracer_err_code_size_exceeded() {
        // TODO NEXT
        unimplemented!()
    }

    fn check_err_invalid_code(
        step: &GethExecStep,
        next_step: Option<&GethExecStep>,
    ) -> bool {
        // TODO: check CallContext is inside Create or Create2
        let offset = step.stack.last().unwrap();
        step.op == OpcodeId::RETURN
            && step.error.is_none()
            && result(next_step) == Word::zero()
            && step.memory.0.len() > 0
            && step.memory.0.get(offset.low_u64() as usize) == Some(&0xef)
    }

    #[test]
    fn tracer_err_invalid_code() {
        let code_creator = bytecode! {
            PUSH32(word!("0xef00000000000000000000000000000000000000000000000000000000000000")) // value
            PUSH1(0x00) // offset
            MSTORE
            PUSH1(0x01) // length
            PUSH1(0x00) // offset
            RETURN
        };

        let code_a = bytecode! {
            PUSH1(0x0) // retLength
            PUSH1(0x0) // retOffset
            PUSH1(0x0) // argsLength
            PUSH1(0x0) // argsOffset
            PUSH1(0x0) // value
            PUSH32(word!("0x0000000000000000000000000000000000000123")) // addr
            PUSH32(0x10_000) // gas
            CALL

            PUSH2(0xaa)
        };

        let mut code_b = Bytecode::default();
        // pad code_creator to multiple of 32 bytes
        let len = code_creator.code().len();
        let code_creator: Vec<u8> = code_creator
            .code()
            .iter()
            .cloned()
            .chain(0u8..((32 - len % 32) as u8))
            .collect();
        for (index, word) in code_creator.chunks(32).enumerate() {
            code_b.push(32, Word::from_big_endian(word));
            code_b.push(32, Word::from(index * 32));
            code_b.write_op(OpcodeId::MSTORE);
        }
        let code_b_end = bytecode! {
            // PUSH1(0xef) // value
            // PUSH1(0x00) // offset
            // MSTORE
            PUSH1(len) // length
            PUSH1(0x00) // offset
            PUSH1(0x00) // value
            CREATE

            PUSH3(0xbb)
        };
        code_b.append(&code_b_end);
        let struct_logs = trace_code_2(&code_a, &code_b);

        // get last RETURN
        let (index, last_step) = struct_logs
            .iter()
            .enumerate()
            .rev()
            .find(|(_, s)| s.op == OpcodeId::RETURN)
            .unwrap();
        // println!("{:#?}", &struct_logs[index - 5..index + 3]);
        assert_eq!(
            check_err_invalid_code(
                &struct_logs[index],
                struct_logs.get(index + 1)
            ),
            true
        );
        // Unfortunately the trace doesn't record errors generated by a CALL.  We only get the
        // success = 0 the next step's stack
        // assert_eq!(last_step.error, None);
        // assert_eq!(struct_logs[index + 1].op, OpcodeId::PUSH2);
        // assert_eq!(struct_logs[index + 1].stack, Stack(vec![Word::from(0)])); // success = 0
    }

    // TODO
    fn check_err_code_store_out_of_gas(
        _step: &GethExecStep,
        _next_step: Option<&GethExecStep>,
    ) -> bool {
        unimplemented!()
    }

    // code store out of gas
    // TODO.  Triggered on create
    #[test]
    fn tracer_err_code_store_out_of_gas() {
        unimplemented!()
    }

    //
    // Errors not reported
    //

    fn result(step: Option<&GethExecStep>) -> Word {
        step.map(|s| s.stack.last().unwrap_or(Word::zero()))
            .unwrap_or(Word::zero())
    }

    fn check_err_invalid_jump(
        step: &GethExecStep,
        next_step: Option<&GethExecStep>,
    ) -> bool {
        let next_depth = next_step.map(|s| s.depth).unwrap_or(0);
        [OpcodeId::JUMP, OpcodeId::JUMPI].contains(&step.op)
            && step.error.is_none()
            && result(next_step) == Word::zero()
            && step.depth != next_depth
    }

    #[test]
    fn tracer_err_invalid_jump() {
        let code = bytecode! {
            PUSH1(0x10)
            JUMP
            STOP
        };
        let index_jump = 1;
        let block = mock::BlockData::new_single_tx_trace_code(&code).unwrap();
        assert_eq!(block.geth_trace.struct_logs.len(), 2);
        assert_eq!(
            check_err_invalid_jump(
                &block.geth_trace.struct_logs[index_jump],
                block.geth_trace.struct_logs.get(index_jump + 1)
            ),
            true
        );
        // println!("{:#?}", block.geth_trace.struct_logs);
        // The error is not found in the GethExecStep. :(

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
        let index_jump = 8;
        let struct_logs = trace_code_2(&code_a, &code);

        assert_eq!(
            check_err_invalid_jump(
                &struct_logs[index_jump],
                struct_logs.get(index_jump + 1)
            ),
            true
        );
    }

    fn check_err_execution_reverted(
        step: &GethExecStep,
        next_step: Option<&GethExecStep>,
    ) -> bool {
        let next_depth = next_step.map(|s| s.depth).unwrap_or(0);
        step.op == OpcodeId::REVERT
            && step.error.is_none()
            && result(next_step) == Word::zero()
            && step.depth != next_depth
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
        let index_revert = 2;
        let block = mock::BlockData::new_single_tx_trace_code(&code).unwrap();
        assert_eq!(block.geth_trace.struct_logs.len(), 3);

        assert_eq!(
            check_err_execution_reverted(
                &block.geth_trace.struct_logs[index_revert],
                block.geth_trace.struct_logs.get(index_revert + 1)
            ),
            true
        );

        let code_a = bytecode! {
            PUSH1(0x0) // retLength
            PUSH1(0x0) // retOffset
            PUSH1(0x0) // argsLength
            PUSH1(0x0) // argsOffset
            PUSH1(0x0) // value
            PUSH32(word!("0x0000000000000000000000000000000000000123")) // addr
            PUSH32(0x10_000) // gas
            CALL

            PUSH2(0xaa)
        };
        let index_jump = 10;
        let struct_logs = trace_code_2(&code_a, &code);

        assert_eq!(
            check_err_execution_reverted(
                &struct_logs[index_jump],
                struct_logs.get(index_jump + 1)
            ),
            true
        );
    }

    fn check_err_return_data_out_of_bounds(
        step: &GethExecStep,
        next_step: Option<&GethExecStep>,
    ) -> bool {
        let next_depth = next_step.map(|s| s.depth).unwrap_or(0);
        step.op == OpcodeId::RETURNDATACOPY
            && step.error.is_none()
            && result(next_step) == Word::zero()
            && step.depth != next_depth
    }

    #[test]
    fn tracer_err_return_data_out_of_bounds() {
        let code_a = bytecode! {
            PUSH1(0x0) // retLength
            PUSH1(0x0) // retOffset
            PUSH1(0x0) // argsLength
            PUSH1(0x0) // argsOffset
            PUSH1(0x0) // value
            PUSH32(word!("0x0000000000000000000000000000000000000123")) // addr
            PUSH32(0x10_000) // gas
            CALL

            PUSH1(0x02) // length
            PUSH1(0x00) // offset
            PUSH1(0x00) // destOffset
            RETURNDATACOPY

            PUSH2(0xaa)
        };
        let code_b = bytecode! {
            PUSH2(0x42) // value
            PUSH2(0x00) // offset
            MSTORE
            PUSH1(0x01) // length
            PUSH1(0x00) // offset
            RETURN
        };
        let struct_logs = trace_code_2(&code_a, &code_b);

        // get last RETURNDATACOPY
        let (index, last_step) = struct_logs
            .iter()
            .enumerate()
            .rev()
            .find(|(_, s)| s.op == OpcodeId::RETURNDATACOPY)
            .unwrap();

        assert_eq!(
            check_err_return_data_out_of_bounds(
                &last_step,
                struct_logs.get(index + 1)
            ),
            true
        )
    }

    //
    // Errors Reported
    //

    #[test]
    fn tracer_err_gas_uint_overflow() {
        let code = bytecode! {
            PUSH32(0x42) // value
            PUSH32(0x1_000_000_000_000_000_000u128) // offset
            MSTORE
        };
        let block = mock::BlockData::new_single_tx_trace_code(&code).unwrap();

        assert_eq!(block.geth_trace.struct_logs[2].op, OpcodeId::MSTORE);
        assert_eq!(
            block.geth_trace.struct_logs[2].error,
            Some(ERR_GAS_UINT_OVERFLOW.to_string())
        );
    }

    #[test]
    fn tracer_err_invalid_opcode() {
        let mut code = bytecode::Bytecode::default();
        code.write_op(OpcodeId::PC);
        code.write(0x0f);
        let block = mock::BlockData::new_single_tx_trace_code(&code).unwrap();
        let last_step = &block.geth_trace.struct_logs
            [block.geth_trace.struct_logs.len() - 1];

        assert_eq!(last_step.op, OpcodeId::INVALID(0x0f));
        assert_eq!(
            last_step.error,
            Some(format!("{}: opcode 0xf not defined", ERR_INVALID_OPCODE))
        );
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
        let struct_logs = trace_code_2(&code_a, &code_b);

        assert_eq!(struct_logs[9].op, OpcodeId::SSTORE);
        assert_eq!(
            struct_logs[9].error,
            Some(ERR_WRITE_PROTECTION.to_string())
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
}
