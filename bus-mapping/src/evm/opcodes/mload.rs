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
        // ctx: &mut TraceContext,
        // exec_step: &mut ExecutionStep,
        // next_steps: &[ExecutionStep],
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
        eth_types::Word,
        evm::{GasCost, OpcodeId, Stack, StackAddress, Storage},
        external_tracer, BlockConstants, ExecutionTrace,
    };
    use pasta_curves::pallas::Scalar;

    #[test]
    fn mload_opcode_impl() -> Result<(), Error> {
        let code = bytecode! {
            .setup_state()

            PUSH1(0x40u64)
            #[start]
            MLOAD
            STOP
        };

        let block_ctants = BlockConstants::default();

        // Get the execution steps from the external tracer
        let obtained_steps = &external_tracer::trace(&block_ctants, &code)?
            [code.get_pos("start")..];

        // Obtained trace computation
        let obtained_exec_trace =
            ExecutionTrace::new(obtained_steps.to_vec(), block_ctants)?;

        let mut ctx = TraceContext::new();

        // Start from the same pc and gas limit
        let mut pc = obtained_steps[0].pc();

        // The memory is the same in both steps as none of them edits the
        // memory of the EVM.
        let mem_map = obtained_steps[0].memory.clone();

        // Generate Step1 corresponding to PUSH1 40
        let mut step_1 = ExecutionStep {
            memory: mem_map.clone(),
            stack: Stack(vec![Word::from(0x40u8)]),
            storage: Storage::empty(),
            instruction: OpcodeId::MLOAD,
            gas: obtained_steps[0].gas(),
            gas_cost: GasCost::FASTEST,
            depth: 1u8,
            pc: pc.inc_pre(),
            gc: ctx.gc,
            bus_mapping_instance: vec![],
        };

        // Add StackOp associated to the 0x40 read from the latest Stack pos.
        ctx.push_op(
            &mut step_1,
            StackOp::new(
                RW::READ,
                StackAddress::from(1023),
                Word::from(0x40u8),
            ),
        );

        // Add the 32 MemoryOp generated from the Memory read at addr 0x40<->0x80 for each byte.
        Word::from(0x80u8)
            .to_be_bytes()
            .iter()
            .enumerate()
            .map(|(idx, byte)| (idx + 0x40, byte))
            .for_each(|(idx, byte)| {
                ctx.push_op(
                    &mut step_1,
                    MemoryOp::new(RW::READ, idx.into(), *byte),
                );
            });

        // Add the last Stack write
        ctx.push_op(
            &mut step_1,
            StackOp::new(
                RW::WRITE,
                StackAddress::from(1023),
                Word::from(0x80u8),
            ),
        );

        // Generate Step1 corresponding to PUSH1 40
        let step_2 = ExecutionStep {
            memory: mem_map,
            stack: Stack(vec![Word::from(0x80u8)]),
            storage: Storage::empty(),
            instruction: OpcodeId::STOP,
            gas: obtained_steps[1].gas(),
            gas_cost: GasCost::ZERO,
            depth: 1u8,
            pc: pc.inc_pre(),
            gc: ctx.gc,
            bus_mapping_instance: vec![],
        };

        // Compare first step bus mapping instance
        assert_eq!(
            obtained_exec_trace[0].bus_mapping_instance,
            step_1.bus_mapping_instance
        );
        // Compare first step entirely
        assert_eq!(obtained_exec_trace[0], step_1);

        // Compare second step bus mapping instance
        assert_eq!(
            obtained_exec_trace[1].bus_mapping_instance,
            step_2.bus_mapping_instance
        );
        // Compare second step entirely
        assert_eq!(obtained_exec_trace[1], step_2);

        // Compare containers
        assert_eq!(obtained_exec_trace.ctx.container, ctx.container);

        Ok(())
    }
}
