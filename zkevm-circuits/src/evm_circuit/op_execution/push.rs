use super::super::{Case, Cell, Constraint, ExecutionStep, Word};
use super::utils::{
    self,
    common_cases::{OutOfGasCase, StackOverflowCase},
    constraint_builder::ConstraintBuilder,
    sum, StateTransition, StateTransitionExpressions,
};
use super::{
    CaseAllocation, CaseConfig, CoreStateInstance, OpExecutionState, OpGadget,
};
use crate::impl_op_gadget;
use crate::util::{Expr, ToWord};
use array_init::array_init;
use bus_mapping::evm::{GasCost, OpcodeId};
use halo2::plonk::Error;
use halo2::{arithmetic::FieldExt, circuit::Region};
use std::convert::TryInto;

static STATE_TRANSITION: StateTransition = StateTransition {
    gc_delta: Some(1), // 1 stack push
    pc_delta: None,    // 1 + PUSHX
    sp_delta: Some(-1),
    gas_delta: Some(GasCost::FASTEST.as_u64()),
    next_memory_size: None,
};
const NUM_PUSHED: usize = 1;

impl_op_gadget!(
    #range
    [
         PUSH1,  PUSH2,  PUSH3,  PUSH4,  PUSH5,  PUSH6,  PUSH7,  PUSH8,
         PUSH9, PUSH10, PUSH11, PUSH12, PUSH13, PUSH14, PUSH15, PUSH16,
        PUSH17, PUSH18, PUSH19, PUSH20, PUSH21, PUSH22, PUSH23, PUSH24,
        PUSH25, PUSH26, PUSH27, PUSH28, PUSH29, PUSH30, PUSH31, PUSH32,
    ]
    PushGadget {
        PushSuccessCase(),
        StackOverflowCase(NUM_PUSHED),
        OutOfGasCase(STATE_TRANSITION.gas_delta.unwrap()),
    }
);

#[derive(Clone, Debug)]
struct PushSuccessCase<F> {
    case_selector: Cell<F>,
    value: Word<F>,
    selectors: [Cell<F>; 32], /* whether its PUSH1, ..., or PUSH32 ([1, 1,
                               * 0, ..., 0] means PUSH2) */
}

impl<F: FieldExt> PushSuccessCase<F> {
    pub(crate) const CASE_CONFIG: &'static CaseConfig = &CaseConfig {
        case: Case::Success,
        num_word: 1,
        num_cell: 32, // 32 selectors
        will_halt: false,
    };

    pub(crate) fn construct(alloc: &mut CaseAllocation<F>) -> Self {
        Self {
            case_selector: alloc.selector.clone(),
            value: alloc.words.pop().unwrap(),
            selectors: array_init(|_| alloc.cells.pop().unwrap()),
        }
    }

    pub(crate) fn constraint(
        &self,
        state_curr: &OpExecutionState<F>,
        state_next: &OpExecutionState<F>,
        name: &'static str,
    ) -> Vec<Constraint<F>> {
        let mut cb = ConstraintBuilder::default();

        // Deduce the number of bytes to push from the 'x' value of 'pushx'
        // The number of bytes starts at 1 for PUSH1
        let num_pushed =
            state_curr.opcode.expr() - (OpcodeId::PUSH1.as_u64() - 1).expr();

        // First selector (for the LSB) always needs to be enabled
        cb.require_equal(self.selectors[0].expr(), 1.expr());
        // Check all other selectors
        for idx in 1..32 {
            // selector can transit from 1 to 0 only once as [1, 1, 1, ...,
            // 0, 0, 0]
            cb.require_boolean(
                self.selectors[idx - 1].expr() - self.selectors[idx].expr(),
            );
            // selectors needs to be 0 or 1
            cb.require_boolean(self.selectors[idx].expr());
            // word byte should be 0 when selector is 0
            cb.require_zero(
                self.value.cells[idx].expr()
                    * (1.expr() - self.selectors[idx].expr()),
            );
        }

        // Sum of selectors needs to be exactly the number of bytes
        // that needs to be pushed.
        cb.require_equal(sum::expr(&self.selectors[..]), num_pushed.clone());

        // Push the value on the stack
        cb.stack_push(self.value.expr());

        // State transitions
        // `pc` needs to be increased by number of bytes pushed + 1
        let mut st = StateTransitionExpressions::new(STATE_TRANSITION.clone());
        st.pc_delta = Some(1.expr() + num_pushed);
        st.constraints(&mut cb, state_curr, state_next);

        // Generate the constraint
        vec![cb.constraint(self.case_selector.expr(), name)]
    }

    fn assign(
        &self,
        region: &mut Region<'_, F>,
        offset: usize,
        state: &mut CoreStateInstance,
        step: &ExecutionStep,
    ) -> Result<(), Error> {
        let num_pushed =
            (step.opcode.as_u8() - OpcodeId::PUSH1.as_u8() + 1) as usize;

        // Input
        self.value
            .assign(region, offset, Some(step.values[0].to_word()))?;

        // Selectors
        self.selectors
            .iter()
            .zip(step.values[1].to_word().iter())
            .map(|(alloc, bit)| {
                alloc.assign(region, offset, Some(F::from(*bit as u64)))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // State transitions
        let mut st = STATE_TRANSITION.clone();
        st.pc_delta = Some(1 + num_pushed);
        st.assign(state);

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::super::super::{
        test::TestCircuit, Case, ExecutionStep, Operation,
    };
    use bus_mapping::{evm::OpcodeId, operation::Target};
    use halo2::dev::MockProver;
    use num::BigUint;
    use pairing::bn256::Fr as Fp;

    macro_rules! try_test_circuit {
        ($execution_steps:expr, $operations:expr, $result:expr) => {{
            let circuit =
                TestCircuit::<Fp>::new($execution_steps, $operations, false);
            let prover = MockProver::<Fp>::run(11, &circuit, vec![]).unwrap();
            assert_eq!(prover.verify(), $result);
        }};
    }

    // TODO: use evm word
    // TODO: add failure cases

    #[test]
    fn push_gadget() {
        try_test_circuit!(
            vec![ExecutionStep {
                opcode: OpcodeId::PUSH2,
                case: Case::Success,
                values: vec![
                    BigUint::from(0x02_03u64),
                    BigUint::from(0x01_01u64),
                ],
            }],
            vec![Operation {
                gc: 1,
                target: Target::Stack,
                is_write: true,
                values: [
                    Fp::zero(),
                    Fp::from(1023),
                    Fp::from(2 + 3),
                    Fp::zero(),
                ]
            }],
            Ok(())
        );
    }
}
