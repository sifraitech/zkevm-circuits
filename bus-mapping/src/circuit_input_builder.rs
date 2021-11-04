#![allow(missing_docs)]

use crate::eth_types::{self, Address, GethExecStep, GethExecTrace};
use crate::evm::GlobalCounter;
use crate::evm::OpcodeId;
use crate::exec_trace::OperationRef;
use crate::operation::container::OperationContainer;
use crate::operation::{Op, Operation};
use crate::{BlockConstants, Error};
use core::fmt::Debug;
// use pasta_curves::arithmetic::FieldExt;

// mock
#[derive(Debug)]
pub struct ExecutionStep {
    pub op: OpcodeId,
    pub gc: GlobalCounter,
    pub bus_mapping_instance: Vec<OperationRef>,
}

impl ExecutionStep {
    pub fn new(geth_step: &GethExecStep, gc: GlobalCounter) -> Self {
        Self {
            op: geth_step.op,
            gc,
            bus_mapping_instance: Vec::new(),
        }
    }
    // pub fn gen_associated_ops(
    //     &mut self,
    //     state_ref: &mut CircuitInputStateRef,
    //     next_steps: &[GethExecStep],
    // ) -> Result<(), Error> {
    //     self.op
    //         .gen_associated_ops(&mut self, &mut state_ref, next_steps)
    // }
}

#[derive(Debug)]
pub struct BlockContext {
    pub gc: GlobalCounter,
}

impl BlockContext {
    pub fn new() -> Self {
        Self {
            gc: GlobalCounter::new(),
        }
    }
}

#[derive(Debug)]
pub struct Block {
    pub constants: BlockConstants,
    pub container: OperationContainer,
    txs: Vec<Transaction>,
}

impl Block {
    pub fn new<TX>(
        eth_block: &eth_types::Block<TX>,
        constants: BlockConstants,
    ) -> Self {
        Self {
            constants,
            container: OperationContainer::new(),
            txs: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct TransactionContext {
    address: Address,
}

impl TransactionContext {
    pub fn new(eth_tx: &eth_types::Transaction) -> Self {
        Self {
            address: Address::from([0; 20]),
        }
    }
}

#[derive(Debug)]
pub struct Transaction {
    steps: Vec<ExecutionStep>,
}

impl Transaction {
    pub fn new(eth_tx: &eth_types::Transaction) -> Self {
        Self { steps: Vec::new() }
    }
}

pub struct CircuitInputStateRef<'a> {
    pub block: &'a mut Block,
    pub block_ctx: &'a mut BlockContext,
    pub tx: &'a mut Transaction,
    pub tx_ctx: &'a mut TransactionContext,
    pub step: &'a mut ExecutionStep,
}

impl<'a> CircuitInputStateRef<'a> {
    /// Push an [`Operation`] into the [`OperationContainer`] with the next [`GlobalCounter`] and
    /// then adds a reference to the stored operation ([`OperationRef`]) inside the bus-mapping
    /// instance of the given [`ExecutionStep`].  Then increase the internal [`GlobalCounter`] by
    /// one.
    pub fn push_op<T: Op>(&mut self, op: T) {
        let op_ref = self
            .block
            .container
            .insert(Operation::new(self.block_ctx.gc.inc_pre(), op));
        self.step.bus_mapping_instance.push(op_ref);
    }
}

#[derive(Debug)]
pub struct CircuitInputBuilder {
    pub block: Block,
    pub block_ctx: BlockContext,
}

impl<'a> CircuitInputBuilder {
    pub fn new<TX>(
        eth_block: eth_types::Block<TX>,
        constants: BlockConstants,
    ) -> Self {
        Self {
            block: Block::new(&eth_block, constants),
            block_ctx: BlockContext::new(),
        }
    }

    pub fn state_ref(
        &'a mut self,
        mut tx: &'a mut Transaction,
        mut tx_ctx: &'a mut TransactionContext,
        mut step: &'a mut ExecutionStep,
    ) -> CircuitInputStateRef {
        CircuitInputStateRef {
            block: &mut self.block,
            block_ctx: &mut self.block_ctx,
            tx,
            tx_ctx,
            step,
        }
    }

    pub fn handle_tx(
        &mut self,
        eth_tx: &eth_types::Transaction,
        geth_trace: &GethExecTrace,
    ) -> Result<(), Error> {
        let mut tx = Transaction::new(&eth_tx);
        let mut tx_ctx = TransactionContext::new(&eth_tx);
        for (index, geth_step) in geth_trace.struct_logs.iter().enumerate() {
            let mut step = ExecutionStep::new(&geth_step, self.block_ctx.gc);
            let mut state_ref = self.state_ref(&mut tx, &mut tx_ctx, &mut step);
            geth_step.op.gen_associated_ops(
                &mut state_ref,
                &geth_trace.struct_logs[index..],
            )?;
            tx.steps.push(step);
        }
        self.block.txs.push(tx);
        Ok(())
    }
}
