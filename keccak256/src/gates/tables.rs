use crate::arith_helpers::{convert_b13_coef, B13, B2, B9};
use crate::common::LANE_SIZE;
use crate::gates::rho_helpers::{get_block_count, BASE_NUM_OF_CHUNKS};
use halo2::{
    arithmetic::FieldExt,
    circuit::Layouter,
    plonk::{Advice, Column, ConstraintSystem, Error, Selector, TableColumn},
    poly::Rotation,
};
use std::convert::TryInto;
use std::marker::PhantomData;

use itertools::Itertools;

#[derive(Debug, Clone)]
pub struct Base13toBase9TableConfig<F> {
    base13: TableColumn,
    base9: TableColumn,
    block_count: TableColumn,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> Base13toBase9TableConfig<F> {
    pub(crate) fn load(
        &self,
        layouter: &mut impl Layouter<F>,
    ) -> Result<(), Error> {
        layouter.assign_table(
            || "13 -> 9",
            |mut table| {
                // Iterate over all possible 13-ary values of size 4
                for (i, b13_chunks) in (0..BASE_NUM_OF_CHUNKS)
                    .map(|_| 0..B13)
                    .multi_cartesian_product()
                    .enumerate()
                {
                    table.assign_cell(
                        || "base 13",
                        self.base13,
                        i,
                        || {
                            Ok(F::from(
                                b13_chunks
                                    .iter()
                                    .fold(0, |acc, x| acc * B13 + *x),
                            ))
                        },
                    )?;

                    table.assign_cell(
                        || "base 9",
                        self.base9,
                        i,
                        || {
                            Ok(F::from(b13_chunks.iter().fold(0, |acc, x| {
                                acc * B9 + convert_b13_coef(*x)
                            })))
                        },
                    )?;
                    table.assign_cell(
                        || "block_count",
                        self.block_count,
                        i,
                        || {
                            Ok(F::from(
                                get_block_count(
                                    b13_chunks.clone().try_into().unwrap(),
                                )
                                .into(),
                            ))
                        },
                    )?;
                }
                Ok(())
            },
        )
    }

    pub(crate) fn configure(
        meta: &mut ConstraintSystem<F>,
        q_enable: Selector,
        base13_coef: Column<Advice>,
        base9_coef: Column<Advice>,
        block_count: Column<Advice>,
        fixed: [TableColumn; 3],
    ) -> Self {
        let config = Self {
            base13: fixed[0],
            base9: fixed[1],
            block_count: fixed[2],
            _marker: PhantomData,
        };

        meta.lookup(|meta| {
            let q_enable = meta.query_selector(q_enable);
            let base13_coef = meta.query_advice(base13_coef, Rotation::cur());
            let base9_coef = meta.query_advice(base9_coef, Rotation::cur());
            let bc = meta.query_advice(block_count, Rotation::cur());

            vec![
                (q_enable.clone() * base13_coef, config.base13),
                (q_enable.clone() * base9_coef, config.base9),
                (q_enable * bc, config.block_count),
            ]
        });
        config
    }
}

/// The table describes all possible combinations of these two variables:
/// - The last input accumulator: `high_value`*(13**64) + `low_value`, and
/// - The last output coef: `convert_b13_coef(high_value + low_value)`
#[derive(Debug, Clone)]
pub struct SpecialChunkTableConfig<F> {
    last_chunk: TableColumn,
    output_coef: TableColumn,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> SpecialChunkTableConfig<F> {
    pub(crate) fn load(
        &self,
        layouter: &mut impl Layouter<F>,
    ) -> Result<(), Error> {
        layouter.assign_table(
            || "Special Chunks",
            |mut table| {
                // Iterate over all possible values less than 13 for both low
                // and high
                let mut offset = 0;
                for i in 0..B13 {
                    for j in 0..(B13 - i) {
                        let (low, high) = (i, j);
                        let last_chunk = F::from(low)
                            + F::from(high)
                                * F::from(B13).pow(&[
                                    LANE_SIZE as u64,
                                    0,
                                    0,
                                    0,
                                ]);
                        let output_coef = F::from(convert_b13_coef(low + high));
                        table.assign_cell(
                            || "last chunk",
                            self.last_chunk,
                            offset,
                            || Ok(last_chunk),
                        )?;
                        table.assign_cell(
                            || "output coef",
                            self.output_coef,
                            offset,
                            || Ok(output_coef),
                        )?;
                        offset += 1;
                    }
                }
                Ok(())
            },
        )
    }

    pub(crate) fn configure(
        meta: &mut ConstraintSystem<F>,
        q_enable: Selector,
        last_chunk_advice: Column<Advice>,
        output_coef_advice: Column<Advice>,
        cols: [TableColumn; 2],
    ) -> Self {
        let config = Self {
            last_chunk: cols[0],
            output_coef: cols[1],
            _marker: PhantomData,
        };
        // Lookup for special chunk conversion
        meta.lookup(|meta| {
            let q_enable = meta.query_selector(q_enable);
            let last_chunk_advice =
                meta.query_advice(last_chunk_advice, Rotation::cur());
            let output_coef_advice =
                meta.query_advice(output_coef_advice, Rotation::cur());

            vec![
                (q_enable.clone() * last_chunk_advice, config.last_chunk),
                (q_enable * output_coef_advice, config.output_coef),
            ]
        });
        config
    }
}

const NUM_OF_CHUNKS: usize = 16;

#[derive(Debug, Clone)]
pub struct FromBinaryTableConfig<F> {
    base2: TableColumn,
    base9: TableColumn,
    base13: TableColumn,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> FromBinaryTableConfig<F> {
    pub(crate) fn load(
        &self,
        layouter: &mut impl Layouter<F>,
    ) -> Result<(), Error> {
        layouter.assign_table(
            || "2 -> (9 and 13)",
            |mut table| {
                // Iterate over all possible binary values of size 16
                for (i, b2_chunks) in (0..NUM_OF_CHUNKS)
                    .map(|_| 0..B2)
                    .multi_cartesian_product()
                    .enumerate()
                {
                    table.assign_cell(
                        || "base 2",
                        self.base2,
                        i,
                        || {
                            Ok(F::from(
                                b2_chunks
                                    .iter()
                                    .fold(0, |acc, x| acc * B2 + *x),
                            ))
                        },
                    )?;

                    table.assign_cell(
                        || "base 9",
                        self.base9,
                        i,
                        || {
                            Ok(F::from(
                                b2_chunks
                                    .iter()
                                    .fold(0, |acc, x| acc * B9 + *x),
                            ))
                        },
                    )?;
                    table.assign_cell(
                        || "base 13",
                        self.base13,
                        i,
                        || {
                            Ok(F::from(
                                b2_chunks
                                    .iter()
                                    .fold(0, |acc, x| acc * B13 + *x),
                            ))
                        },
                    )?;
                }
                Ok(())
            },
        )
    }

    pub(crate) fn configure(
        meta: &mut ConstraintSystem<F>,
        q_enable: Selector,
        base2_coef: Column<Advice>,
        base13_coef: Column<Advice>,
        base9_coef: Column<Advice>,
        fixed: [TableColumn; 3],
    ) -> Self {
        let config = Self {
            base2: fixed[0],
            base9: fixed[1],
            base13: fixed[2],
            _marker: PhantomData,
        };

        meta.lookup(|meta| {
            let q_enable = meta.query_selector(q_enable);

            let base2_coef = meta.query_advice(base2_coef, Rotation::cur());
            let base9_coef = meta.query_advice(base9_coef, Rotation::cur());
            let base13_coef = meta.query_advice(base13_coef, Rotation::cur());

            vec![
                (q_enable.clone() * base2_coef, config.base2),
                (q_enable.clone() * base9_coef, config.base9),
                (q_enable.clone() * base13_coef, config.base13),
            ]
        });
        config
    }
}
