use halo2::circuit::Cell;
use halo2::plonk::{Expression, Instance, Selector};
use halo2::{
    circuit::Region,
    plonk::{Advice, Column, ConstraintSystem, Error},
    poly::Rotation,
};
use pasta_curves::arithmetic::FieldExt;
use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct IotaB13Config<F> {
    q_mixing: Selector,
    state: [Column<Advice>; 25],
    // Contains `is_mixing` flag at Rotation::next() and ROUND_CTANT_B13 at
    // Rotation::cur()
    round_ctant_b13: Column<Advice>,
    round_constants: Column<Instance>,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> IotaB13Config<F> {
    // We assume state is recieved in base-9.
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        state: [Column<Advice>; 25],
        round_ctant_b13: Column<Advice>,
        round_constants: Column<Instance>,
    ) -> IotaB13Config<F> {
        // def iota_b13(state: List[List[int], round_constant_base13: int):
        // state[0][0] += round_constant_base13
        // return state

        // Declare the q_mixing.
        let q_mixing = meta.selector();
        // Enable copy constraints over PI and the Advices.
        meta.enable_equality(round_ctant_b13.into());
        meta.enable_equality(round_constants.into());

        meta.create_gate("iota_b13 gate", |meta| {
            // We do a trick which consists on multiplying an internal selector
            // which is always active by the actual `is_mixing` flag
            // which will then enable or disable the gate.
            let q_enable = {
                // We query the flag value from the`round_ctant_b13` `Advice`
                // column at rotation next and multiply to it
                // the active selector so that we avoid the
                // `PoisonedConstraints` and each gate equation
                // can be satisfied while enforcing the correct gate logic.
                let flag = Expression::Constant(F::one())
                    - meta.query_advice(round_ctant_b13, Rotation::next());
                // Note also that we want to enable the gate when `is_mixing` is
                // false. (flag = 0). Therefore, we need to do
                // `1-flag` in order to enforce this.
                meta.query_selector(q_mixing) * flag
            };

            let state_00 = meta.query_advice(state[0], Rotation::cur())
                + meta.query_advice(round_ctant_b13, Rotation::cur());
            let next_lane = meta.query_advice(state[0], Rotation::next());
            vec![q_enable * (state_00 - next_lane)]
        });

        IotaB13Config {
            q_mixing,
            state,
            round_ctant_b13,
            round_constants,
            _marker: PhantomData,
        }
    }

    /// Doc this
    pub fn copy_state_flag_and_assing_rc(
        &self,
        region: &mut Region<'_, F>,
        mut offset: usize,
        state: [(Cell, F); 25],
        out_state: [F; 25],
        absolute_row: usize,
        flag: (Cell, F),
    ) -> Result<(), Error> {
        // Enable `q_mixing`.
        self.q_mixing.enable(region, offset)?;
        // Copy state at offset + 0
        self.copy_state(region, offset, state)?;
        // Assign round_ctant at offset + 0.
        self.assign_round_ctant_b13(region, offset, absolute_row)?;

        offset += 1;
        // Copy flag at `round_ctant_b9` at offset + 1
        self.copy_flag(region, offset, flag)?;
        // Assign out state at offset + 1
        self.assign_state(region, offset, out_state)
    }

    /// Copies the `[(Cell,F);25]` to the `state` Advice column.
    fn copy_state(
        &self,
        region: &mut Region<'_, F>,
        offset: usize,
        in_state: [(Cell, F); 25],
    ) -> Result<(), Error> {
        for (idx, (cell, value)) in in_state.iter().enumerate() {
            let new_cell = region.assign_advice(
                || format!("copy in_state {}", idx),
                self.state[idx],
                offset,
                || Ok(*value),
            )?;

            region.constrain_equal(*cell, new_cell)?;
        }

        Ok(())
    }

    /// Copies the `is_mixing` flag to the `round_ctant_b13` Advice column.
    fn copy_flag(
        &self,
        region: &mut Region<'_, F>,
        offset: usize,
        flag: (Cell, F),
    ) -> Result<(), Error> {
        let obtained_cell = region.assign_advice(
            || format!("assign is_mixing flag {:?}", flag.1),
            self.round_ctant_b13,
            offset,
            || Ok(flag.1),
        )?;
        region.constrain_equal(flag.0, obtained_cell)?;

        Ok(())
    }

    // Assign `[F;25]` at `state` `Advice` column at the provided offset.
    fn assign_state(
        &self,
        region: &mut Region<'_, F>,
        offset: usize,
        state: [F; 25],
    ) -> Result<(), Error> {
        for (idx, lane) in state.iter().enumerate() {
            region.assign_advice(
                || format!("assign state {}", idx),
                self.state[idx],
                offset,
                || Ok(*lane),
            )?;
        }
        Ok(())
    }

    /// Assigns the ROUND_CONSTANTS_BASE_13 to the `absolute_row` passed as an
    /// absolute instance column. Returns the new offset after the
    /// assigment.
    pub fn assign_round_ctant_b13(
        &self,
        region: &mut Region<'_, F>,
        offset: usize,
        absolute_row: usize,
    ) -> Result<(), Error> {
        region.assign_advice_from_instance(
            || format!("assign round_ctant_b13 {}", absolute_row),
            self.round_constants,
            absolute_row,
            self.round_ctant_b13,
            offset,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{arith_helpers::*, common::*, keccak_arith::*};
    use halo2::circuit::Layouter;
    use halo2::plonk::{Advice, Column, ConstraintSystem, Error};
    use halo2::{circuit::SimpleFloorPlanner, dev::MockProver, plonk::Circuit};
    use itertools::Itertools;
    use pasta_curves::arithmetic::FieldExt;
    use pasta_curves::pallas;
    use pretty_assertions::assert_eq;
    use std::convert::TryInto;
    use std::marker::PhantomData;

    #[test]
    fn test_iota_b13_gate() {
        #[derive(Default)]
        struct MyCircuit<F> {
            in_state: [F; 25],
            out_state: [F; 25],
            // This usize is indeed pointing the exact row of the
            // ROUND_CTANTS_B13 we want to use.
            round_ctant: usize,
            // The flag acts like a selector that turns ON/OFF the gate
            flag: bool,
            _marker: PhantomData<F>,
        }

        impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
            type Config = IotaB13Config<F>;
            type FloorPlanner = SimpleFloorPlanner;

            fn without_witnesses(&self) -> Self {
                Self::default()
            }

            fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
                let state: [Column<Advice>; 25] = (0..25)
                    .map(|_| {
                        let column = meta.advice_column();
                        meta.enable_equality(column.into());
                        column
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                let round_ctant_b9 = meta.advice_column();
                // Allocate space for the round constants in base-13 which is an
                // instance column
                let round_ctants = meta.instance_column();
                IotaB13Config::configure(
                    meta,
                    state,
                    round_ctant_b9,
                    round_ctants,
                )
            }

            fn synthesize(
                &self,
                config: Self::Config,
                mut layouter: impl Layouter<F>,
            ) -> Result<(), Error> {
                let offset: usize = 0;

                let val: F = self.flag.into();
                layouter.assign_region(
                    || "Wittnes & assignation",
                    |mut region| {
                        // Witness `is_missing` flag
                        let cell = region.assign_advice(
                            || "witness is_missing",
                            config.round_ctant_b13,
                            offset + 1,
                            || Ok(val),
                        )?;
                        let flag = (cell, val);

                        // Witness `state`
                        let in_state: [(Cell, F); 25] = {
                            let mut state: Vec<(Cell, F)> =
                                Vec::with_capacity(25);
                            for (idx, val) in self.in_state.iter().enumerate() {
                                let cell = region.assign_advice(
                                    || "witness input state",
                                    config.state[idx],
                                    offset,
                                    || Ok(*val),
                                )?;
                                state.push((cell, *val))
                            }
                            state.try_into().unwrap()
                        };

                        // Assign `in_state`, `out_state`, round and flag
                        config.copy_state_flag_and_assing_rc(
                            &mut region,
                            offset,
                            in_state,
                            self.out_state,
                            self.round_ctant,
                            flag,
                        )?;
                        Ok(())
                    },
                )
            }
        }

        let input1: State = [
            [1, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
        ];
        let mut in_biguint = StateBigInt::default();
        let mut in_state: [pallas::Base; 25] = [pallas::Base::zero(); 25];

        for (x, y) in (0..5).cartesian_product(0..5) {
            in_biguint[(x, y)] = convert_b2_to_b13(input1[x][y]);
            in_state[5 * x + y] = big_uint_to_pallas(&in_biguint[(x, y)]);
        }

        let round_ctant = ROUND_CONSTANTS[PERMUTATION - 1];
        // Compute out state
        let s1_arith = KeccakFArith::iota_b13(&in_biguint, round_ctant);
        let out_state = state_bigint_to_pallas::<pallas::Base, 25>(s1_arith);

        let constants: Vec<pallas::Base> = ROUND_CONSTANTS
            .iter()
            .map(|num| big_uint_to_pallas(&convert_b2_to_b13(*num)))
            .collect();

        // With flag set to false, the gate should trigger.
        {
            // With the correct input and output witnesses, the proof should
            // pass.
            let circuit = MyCircuit::<pallas::Base> {
                in_state,
                out_state,
                round_ctant: PERMUTATION - 1,
                flag: false,
                _marker: PhantomData,
            };

            let prover = MockProver::<pallas::Base>::run(
                9,
                &circuit,
                vec![constants.clone()],
            )
            .unwrap();

            assert_eq!(prover.verify(), Ok(()));

            // With wrong input and/or output witnesses, the proof should fail
            // to be verified.
            let circuit = MyCircuit::<pallas::Base> {
                in_state,
                out_state: in_state,
                round_ctant: PERMUTATION - 1,
                flag: false,
                _marker: PhantomData,
            };

            let prover = MockProver::<pallas::Base>::run(
                9,
                &circuit,
                vec![constants.clone()],
            )
            .unwrap();

            assert!(prover.verify().is_err());
        }

        // With flag set to `true`, the gate shouldn't trigger. And so we can
        // pass any witness data and the proof should pass.
        {
            let circuit = MyCircuit::<pallas::Base> {
                in_state,
                out_state: in_state,
                round_ctant: PERMUTATION - 1,
                flag: true,
                _marker: PhantomData,
            };

            let prover =
                MockProver::<pallas::Base>::run(9, &circuit, vec![constants])
                    .unwrap();

            assert_eq!(prover.verify(), Ok(()));
        }
    }
}
