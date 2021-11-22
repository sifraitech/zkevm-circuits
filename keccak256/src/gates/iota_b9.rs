use crate::arith_helpers::*;
use crate::common::{PERMUTATION, ROUND_CONSTANTS};
use crate::keccak_arith::KeccakFArith;
use halo2::circuit::Cell;
use halo2::plonk::Instance;
use halo2::{
    circuit::Region,
    plonk::{
        Advice, Column, ConstraintSystem, Error, Expression, VirtualCells,
    },
    poly::Rotation,
};
use pasta_curves::arithmetic::FieldExt;
use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct IotaB9Config<F> {
    q_enable: Expression<F>,
    state: [Column<Advice>; 25],
    pub(crate) round_ctant_b9: Column<Advice>,
    pub(crate) round_constants: Column<Instance>,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> IotaB9Config<F> {
    // We assume state is recieved in base-9.
    pub fn configure(
        q_enable_fn: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        meta: &mut ConstraintSystem<F>,
        state: [Column<Advice>; 25],
        round_ctant_b9: Column<Advice>,
        round_constants: Column<Instance>,
    ) -> IotaB9Config<F> {
        let mut q_enable = Expression::Constant(F::zero());
        // Enable copy constraints over PI and the Advices.
        meta.enable_equality(round_ctant_b9.into());
        meta.enable_equality(round_constants.into());
        meta.create_gate("iota_b9", |meta| {
            // def iota_b9(state: List[List[int], round_constant_base9: int):
            //     d = round_constant_base9
            //     # state[0][0] has 2*a + b + 3*c already, now add 2*d to make
            // it 2*a + b + 3*c + 2*d     # coefficient in 0~8
            //     state[0][0] += 2*d
            //     return state
            q_enable = q_enable_fn(meta);
            let state_00 = meta.query_advice(state[0], Rotation::cur())
                + (Expression::Constant(F::from(2))
                    * meta.query_advice(round_ctant_b9, Rotation::cur()));
            let next_lane = meta.query_advice(state[0], Rotation::next());
            vec![q_enable.clone() * (state_00 - next_lane)]
        });
        IotaB9Config {
            q_enable,
            state,
            round_ctant_b9,
            round_constants,
            _marker: PhantomData,
        }
    }

    /// Doc this
    pub fn assing_states_and_rc(
        &self,
        region: &mut Region<'_, F>,
        mut offset: usize,
        in_state: [F; 25],
        out_state: [F; 25],
        absolute_row: usize,
    ) -> Result<(), Error> {
        // Assign state
        self.assign_state(region, offset, in_state)?;

        // Assign round_constant at offset + 0
        self.assign_round_ctant_b9(region, offset, absolute_row)?;

        offset += 1;
        // Assign out_state at offset + 1
        self.assign_state(region, offset, out_state)
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
        // Copy state at offset + 0
        self.copy_state(region, offset, state)?;
        // Assign round_ctant at offset + 0.
        self.assign_round_ctant_b9(region, offset, absolute_row)?;

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

    /// Copies the `is_mixing` flag to the `round_ctant_b9` Advice column.
    fn copy_flag(
        &self,
        region: &mut Region<'_, F>,
        offset: usize,
        flag: (Cell, F),
    ) -> Result<(), Error> {
        let obtained_cell = region.assign_advice(
            || format!("assign is_mixing flag {:?}", flag.1),
            self.round_ctant_b9,
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

    /// Assigns the ROUND_CONSTANTS_BASE_9 to the `absolute_row` passed as an
    /// absolute instance column. Returns the new offset after the
    /// assigment.
    pub fn assign_round_ctant_b9(
        &self,
        region: &mut Region<'_, F>,
        offset: usize,
        absolute_row: usize,
    ) -> Result<(), Error> {
        region.assign_advice_from_instance(
            // `absolute_row` is the absolute offset in the overall Region
            // where the Column is laying.
            || format!("assign round_ctant_b9 {}", absolute_row),
            self.round_constants,
            absolute_row,
            self.round_ctant_b9,
            offset,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::PERMUTATION;
    use super::*;
    use crate::common::*;
    use crate::keccak_arith::*;
    use halo2::circuit::Layouter;
    use halo2::plonk::Selector;
    use halo2::plonk::{Advice, Column, ConstraintSystem, Error};
    use halo2::{circuit::SimpleFloorPlanner, dev::MockProver, plonk::Circuit};
    use itertools::Itertools;
    use pasta_curves::arithmetic::FieldExt;
    use pasta_curves::pallas;
    use pretty_assertions::assert_eq;
    use std::convert::TryInto;
    use std::marker::PhantomData;

    #[test]
    fn test_iota_b9_gate_with_flag() {
        #[derive(Default)]
        struct MyCircuit<F> {
            in_state: [F; 25],
            out_state: [F; 25],
            // This usize is indeed pointing the exact row of the
            // ROUND_CTANTS_B9 we want to use.
            round_ctant: usize,
            // The flag acts like a selector that turns ON/OFF the gate
            flag: bool,
            _marker: PhantomData<F>,
        }

        impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
            type Config = IotaB9Config<F>;
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
                // Allocate space for the round constants in base-9 which is an
                // instance column
                let round_ctants = meta.instance_column();

                // Since we're not using a selector and want to test IotaB9 with
                // the Mixing step, we make q_enable query
                // the round_ctant_b9 at `Rotation::next`.
                IotaB9Config::configure(
                    |meta| meta.query_advice(round_ctant_b9, Rotation::next()),
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
                            config.round_ctant_b9,
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
                            state.try_into().expect("try_into err")
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
            in_biguint[(x, y)] = convert_b2_to_b9(input1[x][y]);
            in_state[5 * x + y] = big_uint_to_pallas(&in_biguint[(x, y)]);
        }

        // Test for all rounds
        for round_ctant in 0..25 {
            // Compute out state
            let s1_arith = KeccakFArith::iota_b9(
                &in_biguint,
                ROUND_CONSTANTS[round_ctant],
            );
            let out_state =
                state_bigint_to_pallas::<pallas::Base, 25>(s1_arith);

            let circuit = MyCircuit::<pallas::Base> {
                in_state,
                out_state,
                round_ctant,
                flag: true,
                _marker: PhantomData,
            };

            let constants: Vec<pallas::Base> = ROUND_CONSTANTS
                .iter()
                .map(|num| big_uint_to_pallas(&convert_b2_to_b9(*num)))
                .collect();

            let prover =
                MockProver::<pallas::Base>::run(9, &circuit, vec![constants])
                    .unwrap();

            assert_eq!(prover.verify(), Ok(()));
        }
    }

    #[test]
    fn test_iota_b9_gate_with_selector() {
        #[derive(Default)]
        struct MyCircuit<F> {
            in_state: [F; 25],
            out_state: [F; 25],
            round_ctant_b9: usize,
            _marker: PhantomData<F>,
        }

        #[derive(Clone)]
        struct MyConfig<F> {
            q_enable: Selector,
            iota_b9_config: IotaB9Config<F>,
        }

        impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
            type Config = MyConfig<F>;
            type FloorPlanner = SimpleFloorPlanner;

            fn without_witnesses(&self) -> Self {
                Self::default()
            }

            fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
                let q_enable = meta.selector();
                let state: [Column<Advice>; 25] = (0..25)
                    .map(|_| meta.advice_column())
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                let round_ctant_b9 = meta.advice_column();
                // Allocate space for the round constants in base-9 which is an
                // instance column
                let round_ctants = meta.instance_column();

                MyConfig {
                    q_enable,
                    iota_b9_config: IotaB9Config::configure(
                        |meta| meta.query_selector(q_enable),
                        meta,
                        state,
                        round_ctant_b9,
                        round_ctants,
                    ),
                }
            }

            fn synthesize(
                &self,
                config: Self::Config,
                mut layouter: impl Layouter<F>,
            ) -> Result<(), Error> {
                let offset: usize = 0;
                // Assign input state at offset + 0
                layouter.assign_region(
                    || "assign input state",
                    |mut region| {
                        // Enable selector at offset = 0
                        config.q_enable.enable(&mut region, offset)?;
                        // Start IotaB9 config without copy at offset = 0
                        config.iota_b9_config.assing_states_and_rc(
                            &mut region,
                            offset,
                            self.in_state,
                            self.out_state,
                            self.round_ctant_b9,
                        )
                    },
                )?;

                Ok(())
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
            in_biguint[(x, y)] = convert_b2_to_b9(input1[x][y]);
            in_state[5 * x + y] = big_uint_to_pallas(&in_biguint[(x, y)]);
        }

        // Test for the 25 rounds
        for round in 0..PERMUTATION {
            // Compute out state
            let s1_arith =
                KeccakFArith::iota_b9(&in_biguint, ROUND_CONSTANTS[round]);
            let out_state =
                state_bigint_to_pallas::<pallas::Base, 25>(s1_arith);

            let circuit = MyCircuit::<pallas::Base> {
                in_state,
                out_state,
                round_ctant_b9: round,
                _marker: PhantomData,
            };

            let constants: Vec<pallas::Base> = ROUND_CONSTANTS
                .iter()
                .map(|num| big_uint_to_pallas(&convert_b2_to_b9(*num)))
                .collect();

            let prover =
                MockProver::<pallas::Base>::run(9, &circuit, vec![constants])
                    .unwrap();

            assert_eq!(prover.verify(), Ok(()));
        }
    }
}
