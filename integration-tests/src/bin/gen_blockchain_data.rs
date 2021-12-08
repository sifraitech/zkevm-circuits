use ethers_solc::Solc;
use integration_tests::{CONTRACTS, CONTRACTS_PATH, GETH0_URL};
use std::fs::File;
use std::path::Path;

fn main() {
    // Compile contracts
    for contract in CONTRACTS {
        let path_base = Path::new(CONTRACTS_PATH).join(contract);
        let mut path_sol = path_base.clone();
        path_sol.set_extension("sol");
        let compiled = Solc::default().compile_source(&path_sol).unwrap();
        if compiled.errors.len() != 0 {
            panic!("Errors compiling {:?}:\n{:#?}", &path_sol, compiled.errors)
        }

        let mut path_json = path_base.clone();
        path_json.set_extension("json");
        serde_json::to_writer(&File::create(path_json).unwrap(), &compiled)
            .unwrap();
    }

    // Wait for geth to be online.
    // TODO

    // Make sure the blockchain is in a clean state: block 0 is the last block.
    // TODO

    // Transfer funds to our account.
    // TODO

    // Deploy smart contracts
    // TODO
}
