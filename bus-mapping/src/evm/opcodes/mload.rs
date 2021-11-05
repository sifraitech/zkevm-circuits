use super::Opcode;
use crate::circuit_input_builder::CircuitInputStateRef;
use crate::eth_types::GethExecStep;
use crate::{
    evm::MemoryAddress,
    // exec_trace::{ExecutionStep, TraceContext},
    operation::{MemoryOp, StackOp, RW},
    Error,
};
use core::convert::TryInto;

/// Placeholder structure used to implement [`Opcode`] trait over it corresponding to the
/// [`OpcodeId::MLOAD`](crate::evm::OpcodeId::MLOAD) `OpcodeId`.
/// This is responsible of generating all of the associated [`StackOp`]s and [`MemoryOp`]s and place them
/// inside the trace's [`OperationContainer`](crate::operation::OperationContainer).
#[derive(Debug, Copy, Clone)]
pub(crate) struct Mload;

impl Opcode for Mload {
    #[allow(unused_variables)]
    fn gen_associated_ops(
        state: &mut CircuitInputStateRef,
        steps: &[GethExecStep],
    ) -> Result<(), Error> {
        let step = &steps[0];
        //
        // First stack read
        //
        let stack_value_read = step.stack.last()?;
        let stack_position = step.stack.last_filled();

        // Manage first stack read at latest stack position
        state.push_op(StackOp::new(RW::READ, stack_position, stack_value_read));

        //
        // First mem read -> 32 MemoryOp generated.
        //
        let mut mem_read_addr: MemoryAddress = stack_value_read.try_into()?;
        let mem_read_value = steps[1].memory.read_word(mem_read_addr)?;

        let mut bytes = [0u8; 32];
        mem_read_value.to_big_endian(&mut bytes);
        bytes.iter().for_each(|value_byte| {
            state.push_op(MemoryOp::new(RW::READ, mem_read_addr, *value_byte));

            // Update mem_read_addr to next byte's one
            mem_read_addr += MemoryAddress::from(1);
        });

        //
        // First stack write
        //
        state.push_op(StackOp::new(RW::WRITE, stack_position, mem_read_value));

        Ok(())
    }
}

#[cfg(test)]
mod mload_tests {
    use super::*;
    use crate::{
        bytecode,
        circuit_input_builder::{
            CircuitInputBuilder, ExecStep, MockBlock, Transaction,
            TransactionContext,
        },
        eth_types::Word,
        evm::StackAddress,
    };

    #[test]
    fn mload_opcode_impl() -> Result<(), Error> {
        let code = bytecode! {
            .setup_state()

            PUSH1(0x40u64)
            #[start]
            MLOAD
            STOP
        };

        // Get the execution steps from the external tracer
        let mut mock = MockBlock::new_single_tx_trace_code(&code).unwrap();
        mock.geth_trace.struct_logs =
            mock.geth_trace.struct_logs[code.get_pos("start")..].to_vec();

        let mut builder = CircuitInputBuilder::new(
            mock.eth_block.clone(),
            mock.block_ctants.clone(),
        );
        builder.handle_tx(&mock.eth_tx, &mock.geth_trace).unwrap();

        let mut test_builder =
            CircuitInputBuilder::new(mock.eth_block, mock.block_ctants.clone());
        let mut tx = Transaction::new(&mock.eth_tx);
        let mut tx_ctx = TransactionContext::new(&mock.eth_tx);

        // Generate step corresponding to MLOAD
        let mut step = ExecStep::new(
            &mock.geth_trace.struct_logs[0],
            test_builder.block_ctx.gc,
        );
        let mut state_ref =
            test_builder.state_ref(&mut tx, &mut tx_ctx, &mut step);

        // Add StackOp associated to the 0x40 read from the latest Stack pos.
        state_ref.push_op(StackOp::new(
            RW::READ,
            StackAddress::from(1023),
            Word::from(0x40u8),
        ));

        // Add the 32 MemoryOp generated from the Memory read at addr 0x40<->0x80 for each byte.
        Word::from(0x80u8)
            .to_be_bytes()
            .iter()
            .enumerate()
            .map(|(idx, byte)| (idx + 0x40, byte))
            .for_each(|(idx, byte)| {
                state_ref.push_op(MemoryOp::new(RW::READ, idx.into(), *byte));
            });

        // Add the last Stack write
        state_ref.push_op(StackOp::new(
            RW::WRITE,
            StackAddress::from(1023),
            Word::from(0x80u8),
        ));
        tx.steps_mut().push(step);
        test_builder.block.txs_mut().push(tx);

        // Compare first step bus mapping instance
        assert_eq!(
            builder.block.txs()[0].steps()[0].bus_mapping_instance,
            test_builder.block.txs()[0].steps()[0].bus_mapping_instance,
        );

        // Compare containers
        assert_eq!(builder.block.container, test_builder.block.container);

        Ok(())
    }
}
