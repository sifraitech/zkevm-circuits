#![allow(missing_docs)]

use crate::evm::GlobalCounter;
use crate::exec_trace::parsing::{GethExecStep, GethExecTrace};
use crate::operation::{container::OperationContainer, EthAddress};
use crate::{BlockConstants, Error};
use core::fmt::Debug;
use pasta_curves::arithmetic::FieldExt;

// mock
#[derive(Debug)]
pub struct GethBlock {}

// mock
#[derive(Debug)]
pub struct GethTransaction {}

// mock
#[derive(Debug)]
pub struct ExecutionStep {}

impl ExecutionStep {
    pub fn new(geth_step: &GethExecStep, gc: GlobalCounter) -> Self {
        Self {}
    }
    pub fn gen_associated_ops(
        &mut self,
        ctx: &mut Context,
        geth_steps: &[GethExecStep],
    ) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct Block<F: FieldExt> {
    constants: BlockConstants<F>,
    ctx: BlockContext,
    txs: Vec<Transaction>,
}

impl<F: FieldExt> Block<F> {
    pub fn new(geth_block: &GethBlock, constants: BlockConstants<F>) -> Self {
        Self {
            constants,
            ctx: BlockContext::new(),
            txs: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct Transaction {
    ctx: TransactionContext,
    steps: Vec<ExecutionStep>,
}

impl Transaction {
    pub fn new(geth_tx: &GethTransaction) -> Self {
        Self {
            ctx: TransactionContext::new(geth_tx),
            steps: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct TransactionContext {
    address: EthAddress,
}

impl TransactionContext {
    pub fn new(tx: &GethTransaction) -> Self {
        Self {
            address: EthAddress([0; 20]),
        }
    }
}

#[derive(Debug)]
pub struct BlockContext {
    gc: GlobalCounter,
    container: OperationContainer,
}

impl BlockContext {
    pub fn new() -> Self {
        Self {
            gc: GlobalCounter::new(),
            container: OperationContainer::new(),
        }
    }
}

pub struct Context<'a> {
    block: &'a BlockContext,
    tx: &'a TransactionContext,
}

#[derive(Debug)]
pub struct CircuitInputBuilder<F: FieldExt + Debug> {
    block: Block<F>,
}

impl<F: FieldExt> CircuitInputBuilder<F> {
    pub fn new(geth_block: GethBlock, constants: BlockConstants<F>) -> Self {
        Self {
            block: Block::new(&geth_block, constants),
        }
    }

    pub fn handle_tx(
        &mut self,
        geth_tx: &GethTransaction,
        geth_trace: &GethExecTrace,
    ) -> Result<(), Error> {
        let mut tx = Transaction::new(&geth_tx);
        let mut ctx = Context {
            block: &self.block.ctx,
            tx: &tx.ctx,
        };
        for (index, geth_step) in geth_trace.struct_logs.iter().enumerate() {
            let mut step = ExecutionStep::new(&geth_step, self.block.ctx.gc);
            step.gen_associated_ops(
                &mut ctx,
                &geth_trace.struct_logs[index..],
            )?;
            tx.steps.push(step);
        }
        self.block.txs.push(tx);
        Ok(())
    }
}
