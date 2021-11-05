// TODO: Remove this
//! Doc this

/*
use crate::eth_types::Word;
use crate::evm::{Gas, GasCost, ProgramCounter};
// use crate::ExecutionStep;
use crate::{error::Error, evm::OpcodeId};
use core::{convert::TryFrom, str::FromStr};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
*/

/*
impl<'a> TryFrom<&ParsedExecutionStep<'a>> for ExecutionStep {
    type Error = Error;

    fn try_from(
        parsed_step: &ParsedExecutionStep<'a>,
    ) -> Result<Self, Self::Error> {
        // Memory part
        let mem_map: Vec<u8> = parsed_step
            .memory
            .as_ref()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|word| Word::from_str(word))
            .collect::<Result<Vec<_>, uint::FromHexError>>()?
            .iter()
            .flat_map(|word| word.to_be_bytes())
            .collect();

        // Stack part
        let stack: Vec<Word> = parsed_step
            .stack
            .iter()
            .map(|word| Word::from_str(word))
            .collect::<Result<_, _>>()?;

        // Storage part
        let storage: HashMap<Word, Word> = parsed_step
            .storage
            .as_ref()
            .unwrap_or(&HashMap::new())
            .iter()
            .map(|(key, value)| -> Result<_, Self::Error> {
                Ok((Word::from_str(key)?, Word::from_str(value)?))
            })
            .collect::<Result<HashMap<Word, Word>, _>>()?;

        Ok(ExecutionStep::new(
            mem_map,
            stack,
            storage,
            // Avoid setting values now. This will be done at the end.
            OpcodeId::from_str(parsed_step.op)?,
            parsed_step.gas,
            parsed_step.gas_cost,
            parsed_step.depth,
            parsed_step.pc,
            0.into(),
        ))
    }
}
*/

// #[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
// #[doc(hidden)]
// pub(crate) struct GethBlock {}
//
// #[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
// #[doc(hidden)]
// pub(crate) struct GethTransaction {}

// /// TODO Corresponds to `StructLogRes` in `go-ethereum/internal/ethapi/api.go`.
// #[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
// #[doc(hidden)]
// pub struct GethExecStep {
//     pub pc: ProgramCounter,
//     pub op: OpcodeId,
//     pub gas: Gas,
//     #[serde(alias = "gasCost")]
//     pub gas_cost: GasCost,
//     pub depth: u8,
//     // pub(crate) error: &'a str,
//     // stack is in hex 0x prefixed
//     // pub stack: Vec<Word>,
//     pub stack: Stack,
//     // memory is in chunks of 32 bytes, in hex
//     #[serde(default)]
//     // pub memory: Vec<Word>,
//     pub memory: Memory,
//     // storage is hex -> hex
//     #[serde(default)]
//     pub storage: HashMap<Word, Word>,
// }
//
// /// TODO Corresponds to `ExecutionResult` in `go-ethereum/internal/ethapi/api.go`
// #[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
// #[doc(hidden)]
// pub struct GethExecTrace {
//     pub gas: Gas,
//     pub failed: bool,
//     // return_value is a hex encoded byte array
//     // #[serde(alias = "returnValue")]
//     // pub(crate) return_value: String,
//     #[serde(alias = "structLogs")]
//     pub struct_logs: Vec<GethExecStep>,
// }

/*
/// Helper structure whose only purpose is to serve as a De/Serialization
/// derivation guide for the serde Derive macro.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[doc(hidden)]
pub(crate) struct ParsedExecutionStep<'a> {
    pub(crate) pc: ProgramCounter,
    pub(crate) op: &'a str,
    pub(crate) gas: Gas,
    #[serde(alias = "gasCost")]
    pub(crate) gas_cost: GasCost,
    pub(crate) depth: u8,
    pub(crate) stack: Vec<&'a str>,
    pub(crate) memory: Option<Vec<&'a str>>,
    pub(crate) storage: Option<HashMap<&'a str, &'a str>>,
}
*/

/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::evm::{
        opcodes::ids::OpcodeId, GlobalCounter, Memory, Stack, Storage,
    };

    macro_rules! word {
        ($word_hex:expr) => {
            Word::from_str(&$word_hex).expect("invalid hex Word")
        };
    }

    macro_rules! word_map {
      () => {
        HashMap::new()
      };
      ($($key_hex:expr => $value_hex:expr),*) => {
        {
          use std::iter::FromIterator;
          HashMap::from_iter([(
                $(word!($key_hex), word!($value_hex)),*
          )])
        }
      }
    }

    #[test]
    fn deserialize_geth_exec_trace() {
        let trace_json = r#"
  {
    "gas": 26809,
    "failed": false,
    "returnValue": "",
    "structLogs": [
      {
        "pc": 0,
        "op": "PUSH1",
        "gas": 22705,
        "gasCost": 3,
        "depth": 1,
        "stack": []
      },
      {
        "pc": 163,
        "op": "SLOAD",
        "gas": 5217,
        "gasCost": 2100,
        "depth": 1,
        "stack": [
          "0x1003e2d2",
          "0x2a",
          "0x0"
        ],
        "storage": {
          "0000000000000000000000000000000000000000000000000000000000000000": "000000000000000000000000000000000000000000000000000000000000006f"
        },
        "memory": [
          "0000000000000000000000000000000000000000000000000000000000000000",
          "0000000000000000000000000000000000000000000000000000000000000000",
          "0000000000000000000000000000000000000000000000000000000000000080"
        ]
      }
    ]
  }
        "#;
        let trace: GethExecTrace = serde_json::from_str(trace_json)
            .expect("json-deserialize GethExecTrace");
        assert_eq!(
            trace,
            GethExecTrace {
                gas: Gas(26809),
                failed: false,
                struct_logs: vec![
                    GethExecStep {
                        pc: ProgramCounter(0),
                        op: OpcodeId::PUSH1,
                        gas: Gas(22705),
                        gas_cost: GasCost(3),
                        depth: 1,
                        stack: vec![],
                        storage: word_map!(),
                        memory: vec![],
                    },
                    GethExecStep {
                        pc: ProgramCounter(163),
                        op: OpcodeId::SLOAD,
                        gas: Gas(5217),
                        gas_cost: GasCost(2100),
                        depth: 1,
                        stack: vec![
                            word!("0x1003e2d2"),
                            word!("0x2a"),
                            word!("0x0")
                        ],
                        storage: word_map!("0x0" => "0x6f"),
                        memory: vec![
                            word!("0x0"),
                            word!("0x0"),
                            word!("0x080")
                        ],
                    }
                ],
            }
        );
    }

    #[test]
    fn parse_single_step() {
        let step_json = r#"
        {
            "pc": 5,
            "op": "JUMPDEST",
            "gas": 82,
            "gasCost": 3,
            "depth": 1,
            "stack": [
                "40"
            ],
            "memory": [
              "0000000000000000000000000000000000000000000000000000000000000000",
              "0000000000000000000000000000000000000000000000000000000000000000",
              "0000000000000000000000000000000000000000000000000000000000000080"
            ]
          }
        "#;

        let step_loaded: ExecutionStep = ExecutionStep::try_from(
            &serde_json::from_str::<ParsedExecutionStep>(step_json)
                .expect("Error on parsing"),
        )
        .expect("Error on conversion");

        let expected_step = {
            let mem_map = Memory(
                [Word::from(0u8), Word::from(0u8), Word::from(0x80u8)]
                    .iter()
                    .flat_map(|w| w.to_be_bytes())
                    .collect(),
            );

            ExecutionStep {
                memory: mem_map,
                stack: Stack(vec![Word::from(0x40u8)]),
                storage: Storage::empty(),
                instruction: OpcodeId::JUMPDEST,
                gas: Gas(82),
                gas_cost: GasCost(3),
                depth: 1,
                pc: ProgramCounter(5),
                gc: GlobalCounter(0),
                bus_mapping_instance: vec![],
            }
        };

        assert_eq!(step_loaded, expected_step)
    }
}
*/
