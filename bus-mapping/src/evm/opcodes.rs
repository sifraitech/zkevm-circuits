//! Definition of each opcode of the EVM.
pub mod ids;
mod mload;
mod push;
mod sload;
mod stop;
use self::push::Push1;
use crate::circuit_input_builder::{CircuitInputStateRef, ExecutionStep};
use crate::eth_types::GethExecStep;
use crate::exec_trace::TraceContext;
use crate::Error;
use core::fmt::Debug;
use ids::OpcodeId;
use mload::Mload;
use sload::Sload;
use stop::Stop;

// /// Generic opcode trait which defines the logic of the
// /// [`Operation`](crate::operation::Operation) that should be generated for an
// /// [`ExecutionStep`](crate::exec_trace::ExecutionStep) depending of the
// /// [`OpcodeId`] it contains.
// pub trait Opcode: Debug {
//     /// Generate the associated [`MemoryOp`](crate::operation::MemoryOp)s,
//     /// [`StackOp`](crate::operation::StackOp)s, and
//     /// [`StorageOp`](crate::operation::StorageOp)s associated to the Opcode
//     /// is implemented for.
//     fn gen_associated_ops(
//         &self,
//         ctx: &mut TraceContext,
//         exec_step: &mut ExecutionStep,
//         next_steps: &[ExecutionStep],
//     ) -> Result<(), Error>;
// }

/// TODO
pub trait Opcode: Debug {
    /// TODO
    fn gen_associated_ops(
        state: &mut CircuitInputStateRef,
        next_steps: &[GethExecStep],
    ) -> Result<(), Error>;
}

type FnGenAssociatedOps = fn(
    state: &mut CircuitInputStateRef,
    next_steps: &[GethExecStep],
) -> Result<(), Error>;

impl OpcodeId {
    fn fn_gen_associated_ops(&self) -> FnGenAssociatedOps {
        match *self {
            OpcodeId::PUSH1 => Push1::gen_associated_ops,
            OpcodeId::MLOAD => Mload::gen_associated_ops,
            OpcodeId::SLOAD => Sload::gen_associated_ops,
            OpcodeId::STOP => Stop::gen_associated_ops,
            _ => unimplemented!(),
        }
    }

    /// TODO
    pub fn gen_associated_ops(
        &self,
        state: &mut CircuitInputStateRef,
        next_steps: &[GethExecStep],
    ) -> Result<(), Error> {
        let fn_gen_associated_ops = self.fn_gen_associated_ops();
        fn_gen_associated_ops(state, next_steps)
    }
}

// This is implemented for OpcodeId so that we can downcast the responsabilities
// to the specific Opcode structure implementations since OpcodeId is a single
// structure with all the OPCODES stated as associated constants.
// Easier to solve with a macro. But leaving here for now until we refactor in a
// future PR.
// impl Opcode for OpcodeId {
//     fn gen_associated_ops(
//         &self,
//         ctx: &mut TraceContext,
//         exec_step: &mut ExecutionStep,
//         next_steps: &[ExecutionStep],
//     ) -> Result<(), Error> {
//         match *self {
//             OpcodeId::PUSH1 => {
//                 Push1 {}.gen_associated_ops(ctx, exec_step, next_steps)
//             }
//             OpcodeId::MLOAD => {
//                 Mload {}.gen_associated_ops(ctx, exec_step, next_steps)
//             }
//             OpcodeId::SLOAD => {
//                 Sload {}.gen_associated_ops(ctx, exec_step, next_steps)
//             }
//             OpcodeId::STOP => {
//                 Stop {}.gen_associated_ops(ctx, exec_step, next_steps)
//             }
//             _ => unimplemented!(),
//         }
//     }
// }
