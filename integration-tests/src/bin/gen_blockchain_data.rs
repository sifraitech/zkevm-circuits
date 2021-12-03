use ethers_solc::Solc;
use integration_tests::GETH0_URL;
use std::path::Path;

fn main() {
    const CONTRACTS_PATH: &str = "contracts";
    const CONTRACTS: &[&str] =
        &["migrations/Migrations.sol", "greeter/Greeter.sol"];
    for contract in CONTRACTS {
        let path = Path::new(CONTRACTS_PATH).join(contract);
        let compiled = Solc::default().compile_source(&path).unwrap();
        if compiled.errors.len() != 0 {
            panic!("Errors compiling {:?}:\n{:#?}", &path, compiled.errors)
        }
    }
}
