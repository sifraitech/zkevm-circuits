//! Doc this

use crate::evm::{EvmWord, GasCost, GasInfo, ProgramCounter};
use crate::ExecutionStep;
use crate::Gas;
use crate::{
    error::{Error, EvmWordParsingError},
    evm::OpcodeId,
};
use core::{convert::TryFrom, str::FromStr};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
            .map(|word| EvmWord::from_str(word))
            .collect::<Result<Vec<_>, EvmWordParsingError>>()?
            .iter()
            .flat_map(|word| word.inner())
            .copied()
            .collect();

        // Stack part
        let stack: Vec<EvmWord> = parsed_step
            .stack
            .iter()
            .map(|word| EvmWord::from_str(word))
            .collect::<Result<_, _>>()?;

        // Storage part
        let storage: HashMap<EvmWord, EvmWord> = parsed_step
            .storage
            .as_ref()
            .unwrap_or(&HashMap::new())
            .iter()
            .map(|(key, value)| -> Result<_, EvmWordParsingError> {
                Ok((EvmWord::from_str(key)?, EvmWord::from_str(value)?))
            })
            .collect::<Result<HashMap<EvmWord, EvmWord>, _>>()?;

        Ok(ExecutionStep::new(
            mem_map,
            stack,
            storage,
            // Avoid setting values now. This will be done at the end.
            OpcodeId::from_str(parsed_step.op)?,
            GasInfo::new(parsed_step.gas, parsed_step.gas_cost),
            parsed_step.depth,
            parsed_step.pc,
            // 0.into(),
        ))
    }
}

/// TODO
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[doc(hidden)]
pub(crate) struct GethExecStep<'a> {
    pub(crate) pc: ProgramCounter,
    // pub(crate) op: OpcodeId,
    pub(crate) op: &'a str,
    pub(crate) gas: Gas,
    #[serde(alias = "gasCost")]
    pub(crate) gas_cost: GasCost,
    pub(crate) depth: u8,
    pub(crate) stack: Vec<EvmWord>,
    pub(crate) memory: Option<Vec<&'a str>>,
    pub(crate) storage: Option<HashMap<EvmWord, EvmWord>>,
}

/// TODO
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[doc(hidden)]
pub(crate) struct GethExecTrace<'a> {
    pub(crate) gas: Gas,
    pub(crate) failed: bool,
    #[serde(alias = "returnValue")]
    pub(crate) return_value: &'a str,
    #[serde(alias = "structLogs")]
    pub(crate) struct_logs: Vec<GethExecStep<'a>>,
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evm::{opcodes::ids::OpcodeId, Memory, Stack, Storage};

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
        "gas": 5605,
        "gasCost": 3,
        "depth": 1,
        "stack": []
      },
      {
        "pc": 2,
        "op": "PUSH1",
        "gas": 5602,
        "gasCost": 3,
        "depth": 1,
        "stack": [
          "0x80"
        ]
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
        }
      }
    ]
  }
        "#;
        let trace: GethExecTrace = serde_json::from_str(trace_json)
            .expect("json-deserialize GethExecTrace");
        println!("{:?}", trace);
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
                EvmWord::from(0u8)
                    .inner()
                    .iter()
                    .chain(EvmWord::from(0u8).inner())
                    .chain(EvmWord::from(0x80u8).inner())
                    .copied()
                    .collect(),
            );

            ExecutionStep {
                memory: mem_map,
                stack: Stack(vec![EvmWord::from(0x40u8)]),
                storage: Storage::empty(),
                instruction: OpcodeId::JUMPDEST,
                gas_info: GasInfo::new(82, GasCost::from(3u8)),
                depth: 1,
                pc: ProgramCounter(5),
                // gc: GlobalCounter(0),
                bus_mapping_instance: vec![],
            }
        };

        assert_eq!(step_loaded, expected_step)
    }
}
