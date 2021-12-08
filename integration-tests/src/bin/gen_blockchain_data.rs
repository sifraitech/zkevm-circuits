use ethers_contract::ContractFactory;
use ethers_core::types::{TransactionRequest, U256};
use ethers_core::utils::WEI_IN_ETHER;
use ethers_middleware::SignerMiddleware;
use ethers_providers::Middleware;
use ethers_signers::Signer;
use ethers_solc::{artifacts::CompactContractRef, Solc};
use integration_tests::{get_provider, get_wallet, CONTRACTS, CONTRACTS_PATH};
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

#[tokio::main]
async fn main() {
    // Compile contracts
    let mut compiled_contracts = Vec::new();
    for contract in CONTRACTS {
        let path_base = Path::new(CONTRACTS_PATH).join(contract);
        let mut path_sol = path_base.clone();
        path_sol.set_extension("sol");
        let compiled = Solc::default()
            .compile_source(&path_sol)
            .expect("solc compile error");
        if compiled.errors.len() != 0 {
            panic!("Errors compiling {:?}:\n{:#?}", &path_sol, compiled.errors)
        }

        let mut path_json = path_base.clone();
        path_json.set_extension("json");
        serde_json::to_writer(
            &File::create(path_json).expect("cannot create file"),
            &compiled,
        )
        .expect("cannot serialize json into file");

        compiled_contracts.push(compiled);
    }

    let prov = get_provider();

    // Wait for geth to be online.
    loop {
        match prov.client_version().await {
            Ok(version) => {
                println!("Geth online: {}", version);
                break;
            }
            Err(err) => {
                println!("Geth not available: {:?}", err);
                sleep(Duration::from_millis(500));
            }
        }
    }

    // Make sure the blockchain is in a clean state: block 0 is the last block.
    /*
    let block_number = prov
        .get_block_number()
        .await
        .expect("cannot get block number");
    if block_number.as_u64() != 0 {
        panic!(
            "Blockchain is not in a clean state.  Last block number: {}",
            block_number
        );
    }
    */

    // Transfer funds to our account.
    let accounts = prov.get_accounts().await.expect("cannot get accounts");
    let wallet0 = get_wallet(0);

    let tx = TransactionRequest::new()
        .to(wallet0.address())
        .value(WEI_IN_ETHER * 1)
        .from(accounts[0]);

    prov.send_transaction(tx, None)
        .await
        .expect("cannot send tx")
        .await
        .expect("cannot confirm tx");

    // Deploy smart contracts
    let prov_wallet = SignerMiddleware::new(prov, wallet0);
    let prov_wallet = Arc::new(prov_wallet);
    // let prov = Arc::new(prov);
    for compiled in compiled_contracts {
        for (name, contract) in compiled.contracts_iter() {
            println!("Deploying {}...", name);
            let contract = CompactContractRef::from(contract);
            let factory = ContractFactory::new(
                contract.abi.expect("no abi found").clone(),
                contract.bin.expect("no bin found").clone(),
                prov_wallet.clone(),
                // prov.clone(),
            );

            let deployer = factory
                .deploy(U256::from(42))
                .expect("cannot deploy")
                .confirmations(0usize);
            println!("tx:\n{:#?}", deployer.tx);
            let contract =
                deployer.send().await.expect("cannot confirm deploy");
            println!("{}", contract.address());
        }
    }
}
