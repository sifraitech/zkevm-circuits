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
pub struct ExecStep {
    pub op: OpcodeId,
    pub gc: GlobalCounter,
    pub bus_mapping_instance: Vec<OperationRef>,
}

impl ExecStep {
    pub fn new(geth_step: &GethExecStep, gc: GlobalCounter) -> Self {
        ExecStep {
            op: geth_step.op,
            gc,
            bus_mapping_instance: Vec::new(),
        }
    }
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
        _eth_block: &eth_types::Block<TX>,
        constants: BlockConstants,
    ) -> Self {
        Self {
            constants,
            container: OperationContainer::new(),
            txs: Vec::new(),
        }
    }

    /// TODO
    pub fn txs(&self) -> &[Transaction] {
        &self.txs
    }

    #[cfg(test)]
    pub fn txs_mut(&mut self) -> &mut Vec<Transaction> {
        &mut self.txs
    }
}

#[derive(Debug)]
pub struct CallContext {
    address: Address,
}

#[derive(Debug)]
pub struct TransactionContext {
    call_ctxs: Vec<CallContext>,
}

impl TransactionContext {
    pub fn new(eth_tx: &eth_types::Transaction) -> Self {
        let mut call_ctxs = Vec::new();
        if let Some(addr) = eth_tx.to {
            call_ctxs.push(CallContext { address: addr });
        }
        Self { call_ctxs }
    }
}

#[derive(Debug)]
pub struct Transaction {
    steps: Vec<ExecStep>,
}

impl Transaction {
    pub fn new(_eth_tx: &eth_types::Transaction) -> Self {
        Self { steps: Vec::new() }
    }

    /// TODO
    pub fn steps(&self) -> &[ExecStep] {
        &self.steps
    }

    #[cfg(test)]
    pub fn steps_mut(&mut self) -> &mut Vec<ExecStep> {
        &mut self.steps
    }
}

pub struct CircuitInputStateRef<'a> {
    pub block: &'a mut Block,
    pub block_ctx: &'a mut BlockContext,
    pub tx: &'a mut Transaction,
    pub tx_ctx: &'a mut TransactionContext,
    pub step: &'a mut ExecStep,
}

impl<'a> CircuitInputStateRef<'a> {
    /// Push an [`Operation`] into the [`OperationContainer`] with the next [`GlobalCounter`] and
    /// then adds a reference to the stored operation ([`OperationRef`]) inside the bus-mapping
    /// instance of the current [`ExecStep`].  Then increase the block_ctx [`GlobalCounter`] by
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
        tx: &'a mut Transaction,
        tx_ctx: &'a mut TransactionContext,
        step: &'a mut ExecStep,
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
            let mut step = ExecStep::new(&geth_step, self.block_ctx.gc);
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

use crate::bytecode::Bytecode;
use crate::evm::Gas;
use crate::external_tracer;

/// MockBlock is a type that contains all the information from a block required to build the
/// circuit inputs.
pub struct MockBlock {
    pub eth_block: eth_types::Block<()>,
    pub eth_tx: eth_types::Transaction,
    pub block_ctants: BlockConstants,
    pub geth_trace: eth_types::GethExecTrace,
}

impl MockBlock {
    /// Create a new block with a single tx that executes the code passed by argument.  The trace
    /// will be generated automatically with the external_tracer from the code.
    pub fn new_single_tx_trace_code(code: &Bytecode) -> Result<Self, Error> {
        let eth_block = eth_types::Block::mock();
        let eth_tx = eth_types::Transaction::mock(&eth_block);
        let block_ctants = BlockConstants::from_eth_block(
            &eth_block,
            &eth_types::Word::one(),
            &crate::address!("0x00000000000000000000000000000000c014ba5e"),
        );
        let tracer_tx = external_tracer::Transaction::from_eth_tx(&eth_tx);
        let tracer_account = external_tracer::Account::mock(&code);
        let geth_trace = eth_types::GethExecTrace {
            gas: Gas(eth_tx.gas.as_u64()),
            failed: false,
            struct_logs: external_tracer::trace(
                &block_ctants,
                &tracer_tx,
                &[tracer_account],
            )?
            .to_vec(),
        };
        Ok(Self {
            eth_block,
            eth_tx,
            block_ctants,
            geth_trace,
        })
    }

    /// Create a new block with a single tx that leads to the geth_steps passed by argument.
    pub fn new_single_tx_geth_steps(
        geth_steps: Vec<eth_types::GethExecStep>,
    ) -> Self {
        let eth_block = eth_types::Block::mock();
        let eth_tx = eth_types::Transaction::mock(&eth_block);
        let block_ctants = BlockConstants::from_eth_block(
            &eth_block,
            &eth_types::Word::one(),
            &crate::address!("0x00000000000000000000000000000000c014ba5e"),
        );
        let geth_trace = eth_types::GethExecTrace {
            gas: Gas(eth_tx.gas.as_u64()),
            failed: false,
            struct_logs: geth_steps,
        };
        Self {
            eth_block,
            eth_tx,
            block_ctants,
            geth_trace,
        }
    }
}
