use lazy_static::lazy_static;
use std::env::{self, VarError};

pub const CONTRACTS_PATH: &str = "contracts";
pub const CONTRACTS: &[&str] = &["greeter/Greeter"];

const GETH0_URL_DEFAULT: &str = "http://localhost:8545";

lazy_static! {
    pub static ref GETH0_URL: String = match env::var("GETH0_URL") {
        Ok(val) => val,
        Err(VarError::NotPresent) => GETH0_URL_DEFAULT.to_string(),
        Err(e) => panic!("Error in GETH0_URL env var: {:?}", e),
    };
}
