use super::super::{Case, Cell, Constraint, ExecutionStep};
use super::utils::{
    self, common_cases::OutOfGasCase, constraint_builder::ConstraintBuilder,
    StateTransition,
};
use super::{
    CaseAllocation, CaseConfig, CoreStateInstance, OpExecutionState, OpGadget,
};
use crate::impl_op_gadget;
use crate::util::Expr;
use bus_mapping::evm::{GasCost, OpcodeId};
use halo2::plonk::Error;
use halo2::{arithmetic::FieldExt, circuit::Region};
use std::convert::TryInto;

static STATE_TRANSITION: StateTransition = StateTransition {
    gc_delta: Some(0),
    pc_delta: Some(1),
    sp_delta: Some(0),
    gas_delta: Some(GasCost::ONE.as_u64()),
    next_memory_size: None,
};

impl_op_gadget!(
    #set[JUMPDEST]
    JumpdestGadget {
        JumpdestSuccessCase(),
        OutOfGasCase(STATE_TRANSITION.gas_delta.unwrap()),
    }
);

#[derive(Clone, Debug)]
struct JumpdestSuccessCase<F> {
    case_selector: Cell<F>,
}

impl<F: FieldExt> JumpdestSuccessCase<F> {
    pub(crate) const CASE_CONFIG: &'static CaseConfig = &CaseConfig {
        case: Case::Success,
        num_word: 0,
        num_cell: 0,
        will_halt: false,
    };

    pub(crate) fn construct(alloc: &mut CaseAllocation<F>) -> Self {
        Self {
            case_selector: alloc.selector.clone(),
        }
    }

    pub(crate) fn constraint(
        &self,
        state_curr: &OpExecutionState<F>,
        state_next: &OpExecutionState<F>,
        name: &'static str,
    ) -> Vec<Constraint<F>> {
        let mut cb = ConstraintBuilder::default();

        // State transitions
        STATE_TRANSITION.constraints(&mut cb, state_curr, state_next);

        // Generate the constraint
        vec![cb.constraint(self.case_selector.expr(), name)]
    }

    fn assign(
        &self,
        _region: &mut Region<'_, F>,
        _offset: usize,
        state: &mut CoreStateInstance,
        _step: &ExecutionStep,
    ) -> Result<(), Error> {
        // State transitions
        STATE_TRANSITION.assign(state);

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
    fn jumpdest_gadget() {
        try_test_circuit!(
            vec![
                ExecutionStep {
                    opcode: OpcodeId::PUSH2,
                    case: Case::Success,
                    values: vec![
                        BigUint::from(0x02_03u64),
                        BigUint::from(0x01_01u64),
                    ],
                },
                ExecutionStep {
                    // jumpdest
                    opcode: OpcodeId::JUMPDEST,
                    case: Case::Success,
                    values: vec![],
                }
            ],
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
