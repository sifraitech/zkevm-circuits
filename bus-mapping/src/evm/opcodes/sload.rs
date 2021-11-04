use super::Opcode;
use crate::circuit_input_builder::CircuitInputStateRef;
use crate::eth_types::GethExecStep;
use crate::{
    eth_types::Address,
    exec_trace::{ExecutionStep, TraceContext},
    operation::{StackOp, StorageOp, RW},
    Error,
};

/// Placeholder structure used to implement [`Opcode`] trait over it corresponding to the
/// [`OpcodeId::SLOAD`](crate::evm::OpcodeId::SLOAD) `OpcodeId`.
#[derive(Debug, Copy, Clone)]
pub(crate) struct Sload;

impl Opcode for Sload {
    #[allow(unused_variables)]
    fn gen_associated_ops(
        state: &mut CircuitInputStateRef,
        steps: &[GethExecStep],
        // &self,
        // ctx: &mut TraceContext,
        // exec_step: &mut ExecutionStep,
        // next_steps: &[ExecutionStep],
    ) -> Result<(), Error> {
        let step = &steps[0];
        let gc_start = state.block_ctx.gc;

        // First stack read
        let stack_value_read = step.stack.last()?;
        let stack_position = step.stack.last_filled();

        // Manage first stack read at latest stack position
        state.push_op(StackOp::new(RW::READ, stack_position, stack_value_read));

        // Storage read
        let storage_value_read = step.storage.get_or_err(&stack_value_read)?;
        state.push_op(StorageOp::new(
            RW::READ,
            Address::from([0u8; 20]), // TODO: Fill with the correct value
            stack_value_read,
            storage_value_read,
            storage_value_read,
        ));

        // First stack write
        state.push_op(StackOp::new(
            RW::WRITE,
            stack_position,
            storage_value_read,
        ));

        Ok(())
    }
}

#[cfg(test)]
mod sload_tests {
    use super::*;
    use crate::eth_types::{word, word_map};
    use crate::{
        bytecode,
        eth_types::Word,
        evm::{GasCost, Memory, OpcodeId, Stack, StackAddress, Storage},
        external_tracer, BlockConstants, ExecutionTrace,
    };
    use pasta_curves::pallas::Scalar;
    use std::collections::HashMap;
    use std::iter::FromIterator;

    #[test]
    fn sload_opcode_impl() -> Result<(), Error> {
        let code = bytecode! {
            // Write 0x6f to storage slot 0
            PUSH1(0x6fu64)
            PUSH1(0x00u64)
            SSTORE

            // Load storage slot 0
            PUSH1(0x00u64)
            #[start]
            SLOAD
            STOP
        };

        // Get the execution steps from the external tracer
        // Obtained trace computation
        let eth_block = eth_types::Block::mock();
        let eth_tx = eth_types::Transaction::mock(&geth_block);
        let block_ctants = BlockConstants::from_eth_block(
            &eth_block,
            &Word::one(),
            &Address::from_str("0x00000000000000000000000000000000c014ba5e")
                .unwrap(),
        );
        let tracer_tx = Transaction::from_eth_tx(&eth_tx);
        let tracer_account = Account::mock(&code);
        let geth_trace = GethExecTrace {
            gas: Gas(eth_tx.gas.as_u64()),
            failed: false,
            struct_logs: external_tracer::trace(
                &block_ctants,
                &tracer_tx,
                &[tracer_account],
            )?[code.get_pos("start")..],
        };
        let obtained_steps = &geth_trace.struct_logs;

        let mut builder = CircuitInputBuilder(eth_block, block_ctants.clone());
        builder.handle_tx(eth_tx, geth_trace).unwrap();

        let mut test_builder =
            CircuitInputBuilder(eth_bloc, block_ctants.clone());
        let mut tx = Transaction::new(&eth_tx);
        let mut tx_ctx = TransactionContext::new(&eth_tx);

        // Start from the same pc and gas limit
        let mut pc = geth_trace.struct_logs[0].pc;
        let gas = geth_trace.struct_logs[0].gas;

        // Generate Step1 corresponding to SLOAD
        // let mut step_1 = ExecutionStep {
        //     memory: Memory::new(),
        //     stack: Stack(vec![Word(0x0)]),
        //     storage: Storage(word_map!("0x0" => "0x6f")),
        //     instruction: OpcodeId::SLOAD,
        //     gas,
        //     gas_cost: GasCost::WARM_STORAGE_READ_COST,
        //     depth: 1u8,
        //     pc: pc.inc_pre(),
        //     gc: ctx.gc,
        //     bus_mapping_instance: vec![],
        // };

        // Add StackOp associated to the stack pop.
        let mut step =
            ExecutionStep::new(&geth_trace.struct_logs[0], self.block_ctx.gc);
        let mut state_ref =
            test_builder.state_ref(&mut tx, &mut tx_ctx, &mut step);
        state_ref.push_op(StackOp::new(
            RW::READ,
            StackAddress::from(1023),
            Word::from(0x0u32),
        ));
        // Add StorageOp associated to the storage read.
        state_ref.push_op(StorageOp::new(
            RW::READ,
            Address::from([0u8; 20]), // TODO: Fill with the correct value
            Word::from(0x0u32),
            Word::from(0x6fu32),
            Word::from(0x6fu32),
        ));
        // Add StackOp associated to the stack push.
        state_ref.push_op(StackOp::new(
            RW::WRITE,
            StackAddress::from(1023),
            Word::from(0x6fu32),
        ));

        assert_eq!(
            builder.block.txs[0].steps[0].bus_mapping_instance,
            test_builder.block.txs[0].steps[0].bus_mapping_instance
        );
        // assert_eq!(obtained_exec_trace[0], step_1);
        assert_eq!(builder.block.container, test_builder.block.container);

        Ok(())
    }
}
