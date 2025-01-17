use super::super::{
    Case, Cell, Constraint, CoreStateInstance, ExecutionStep, Word,
};
use super::utils::{
    self,
    common_cases::{OutOfGasCase, StackUnderflowCase},
    constraint_builder::ConstraintBuilder,
    from_bytes,
    math_gadgets::IsZeroGadget,
    select, sum, StateTransition, StateTransitionExpressions,
};

use super::{CaseAllocation, CaseConfig, OpExecutionState, OpGadget};
use crate::impl_op_gadget;
use crate::util::{Expr, ToWord};
use bus_mapping::evm::{GasCost, OpcodeId};
use halo2::plonk::Error;
use halo2::{arithmetic::FieldExt, circuit::Region};
use num::BigUint;
use num::ToPrimitive;
use std::convert::TryInto;

static STATE_TRANSITION: StateTransition = StateTransition {
    gc_delta: Some(2),
    pc_delta: Some(1), // pc is dynamic for jumpi, just set 1 for initialization
    sp_delta: Some(2),
    gas_delta: Some(GasCost::SLOW.as_u64()),
    next_memory_size: None,
};

const NUM_POPPED: usize = 2;
impl_op_gadget!(
    #set[JUMPI]
    JumpiGadget {
        JumpiSuccessCase(),
        OutOfGasCase(STATE_TRANSITION.gas_delta.unwrap()),
        StackUnderflowCase(NUM_POPPED),
        //TODO: ErrJumpcase
    }
);

#[derive(Clone, Debug)]
struct JumpiSuccessCase<F> {
    case_selector: Cell<F>,
    code_hash: Word<F>,
    dest: Word<F>,
    cond: Word<F>,
    is_cond_zero: IsZeroGadget<F>,
}

impl<F: FieldExt> JumpiSuccessCase<F> {
    pub(crate) const CASE_CONFIG: &'static CaseConfig = &CaseConfig {
        case: Case::Success,
        num_word: 3, // two stack values + code_hash of contract
        num_cell: IsZeroGadget::<F>::NUM_CELLS,
        will_halt: false,
    };

    pub(crate) fn construct(alloc: &mut CaseAllocation<F>) -> Self {
        Self {
            case_selector: alloc.selector.clone(),
            code_hash: alloc.words.pop().unwrap(),
            dest: alloc.words.pop().unwrap(),
            cond: alloc.words.pop().unwrap(),
            is_cond_zero: IsZeroGadget::construct(alloc),
        }
    }

    pub(crate) fn constraint(
        &self,
        state_curr: &OpExecutionState<F>,
        state_next: &OpExecutionState<F>,
        name: &'static str,
    ) -> Vec<Constraint<F>> {
        let mut constrains = Vec::<Constraint<F>>::new();
        let mut cb = ConstraintBuilder::default();

        let is_cond_met = 1.expr()
            - self
                .is_cond_zero
                .constraints(&mut cb, sum::expr(&self.cond.cells));

        // State transitions
        let mut st = StateTransitionExpressions::new(STATE_TRANSITION.clone());
        // is_cond_met == 0 --> cond = 0, pc + 1
        // is_cond_met == 1 --> cond != 0, pc = `dest`
        st.pc_delta = Some(select::expr(
            is_cond_met.clone(),
            from_bytes::expr(self.dest.cells[..3].to_vec())
                - state_curr.program_counter.expr(),
            1.expr(),
        ));

        st.constraints(&mut cb, state_curr, state_next);

        // Pop the 'dest' and 'cond' from the stack
        cb.stack_pop(self.dest.expr());
        cb.stack_pop(self.cond.expr());

        // Generate the constraint
        // 1. `cond` is zero constraint (is_cond_met = 0 )
        constrains.push(cb.constraint(
            self.case_selector.expr() * (1.expr() - is_cond_met.clone()),
            name,
        ));
        // 2. `cond` is non-zero constraint (is_cond_met = 1 )
        // lookup byte code table to ensure 'dest' is valid( jumpdest & is_code)
        cb.add_bytecode_lookup([
            self.code_hash.expr(),
            self.dest.expr(),
            1.expr(),
            OpcodeId::JUMPDEST.as_u8().expr(),
        ]);
        constrains
            .push(cb.constraint(self.case_selector.expr() * is_cond_met, name));
        constrains
    }

    fn assign(
        &self,
        region: &mut Region<'_, F>,
        offset: usize,
        state: &mut CoreStateInstance,
        step: &ExecutionStep,
    ) -> Result<(), Error> {
        // Inputs

        self.code_hash.assign(
            region,
            offset,
            Some(step.values[0].to_word()),
        )?;

        self.dest
            .assign(region, offset, Some(step.values[1].to_word()))?;

        self.cond
            .assign(region, offset, Some(step.values[2].to_word()))?;

        self.is_cond_zero.assign(
            region,
            offset,
            sum::value(&step.values[2].to_word()),
        )?;

        // State transitions
        let st = STATE_TRANSITION.clone();
        st.assign(state);
        // other than normal op code, jumpi change pc specially, adjust here
        if step.values[2] != BigUint::from(0x00u64) {
            state.program_counter = step.values[1].to_usize().unwrap();
        }

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

    #[test]
    fn jumpi_gadget_with_nonzero_cond() {
        try_test_circuit!(
            vec![
                ExecutionStep {
                    opcode: OpcodeId::PUSH1,
                    case: Case::Success,
                    values: vec![
                        BigUint::from(0x01u64),
                        BigUint::from(0x01u64),
                    ],
                },
                ExecutionStep {
                    opcode: OpcodeId::PUSH1,
                    case: Case::Success,
                    values: vec![
                        BigUint::from(0x05u64),
                        BigUint::from(0x01u64),
                    ],
                },
                ExecutionStep {
                    // Jumpi
                    opcode: OpcodeId::JUMPI,
                    case: Case::Success,
                    values: vec![
                        BigUint::from(0x00u64), // code hash
                        BigUint::from(0x05u64), // dest
                        BigUint::from(0x01u64), // cond
                    ],
                },
                ExecutionStep {
                    // JUMPDEST
                    opcode: OpcodeId::JUMPDEST,
                    case: Case::Success,
                    values: vec![],
                }
            ],
            vec![
                Operation {
                    gc: 1,
                    target: Target::Stack,
                    is_write: true,
                    values: [
                        Fp::zero(),
                        Fp::from(1023),
                        Fp::from(1u64),
                        Fp::zero(),
                    ]
                },
                Operation {
                    gc: 2,
                    target: Target::Stack,
                    is_write: true,
                    values: [
                        Fp::zero(),
                        Fp::from(1022),
                        Fp::from(5u64),
                        Fp::zero(),
                    ]
                },
                Operation {
                    gc: 3,
                    target: Target::Stack,
                    is_write: false,
                    values: [
                        Fp::zero(),
                        Fp::from(1022),
                        Fp::from(5u64),
                        Fp::zero(),
                    ]
                },
                Operation {
                    gc: 4,
                    target: Target::Stack,
                    is_write: false,
                    values: [
                        Fp::zero(),
                        Fp::from(1023),
                        Fp::from(1u64),
                        Fp::zero(),
                    ]
                }
            ],
            Ok(())
        );
    }
    #[test]
    fn jumpi_gadget_with_zero_cond() {
        try_test_circuit!(
            vec![
                ExecutionStep {
                    opcode: OpcodeId::PUSH1,
                    case: Case::Success,
                    values: vec![
                        BigUint::from(0x00u64),
                        BigUint::from(0x01u64),
                    ],
                },
                ExecutionStep {
                    opcode: OpcodeId::PUSH1,
                    case: Case::Success,
                    values: vec![
                        BigUint::from(0x05u64),
                        BigUint::from(0x01u64),
                    ],
                },
                ExecutionStep {
                    // Jumpi
                    opcode: OpcodeId::JUMPI,
                    case: Case::Success,
                    values: vec![
                        BigUint::from(0x00u64), // code hash
                        BigUint::from(0x05u64), // dest
                        BigUint::from(0x00u64), // cond
                    ],
                },
                /* ExecutionStep {
                 *     // JUMPDEST
                 *     opcode: OpcodeId::JUMPDEST,
                 *     case: Case::Success,
                 *     values: vec![],
                 * } */
            ],
            vec![
                Operation {
                    gc: 1,
                    target: Target::Stack,
                    is_write: true,
                    values: [
                        Fp::zero(),
                        Fp::from(1023),
                        Fp::from(0),
                        Fp::zero(),
                    ]
                },
                Operation {
                    gc: 2,
                    target: Target::Stack,
                    is_write: true,
                    values: [
                        Fp::zero(),
                        Fp::from(1022),
                        Fp::from(5u64),
                        Fp::zero(),
                    ]
                },
                Operation {
                    gc: 3,
                    target: Target::Stack,
                    is_write: false,
                    values: [
                        Fp::zero(),
                        Fp::from(1022),
                        Fp::from(5u64),
                        Fp::zero(),
                    ]
                },
                Operation {
                    gc: 4,
                    target: Target::Stack,
                    is_write: false,
                    values: [
                        Fp::zero(),
                        Fp::from(1023),
                        Fp::from(0u64),
                        Fp::zero(),
                    ]
                }
            ],
            Ok(())
        );
    }
}
