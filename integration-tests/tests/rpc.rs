#![cfg(feature = "test_rpc")]

use bus_mapping::eth_types::{Address, EIP1186ProofResponse, Hash, Word};
use bus_mapping::evm::ProgramCounter;
use bus_mapping::rpc::{BlockNumber, GethClient};
use ethers_providers::Http;
use integration_tests::GETH0_URL;
use std::env::{self, VarError};
use std::str::FromStr;
use url::Url;

fn get_provider() -> GethClient<Http> {
    let transport = Http::new(Url::parse(&GETH0_URL).unwrap());
    GethClient::new(transport)
}

#[tokio::test]
async fn test_get_block_by_number() {
    let prov = get_provider();
    let hash = Hash::from_str(
        "0xe4f7aa19a76fcf31a6adff3b400300849e39dd84076765fb3af09d05ee9d787a",
    )
    .unwrap();
    let block_by_num_latest =
        prov.get_block_by_number(BlockNumber::Latest).await.unwrap();
    assert!(hash == block_by_num_latest.hash.unwrap());
    let block_by_num = prov.get_block_by_number(1u64.into()).await.unwrap();
    assert!(
        block_by_num.transactions[0].hash
            == block_by_num_latest.transactions[0].hash
    );
}

#[tokio::test]
async fn test_get_block_by_hash() {
    let prov = get_provider();

    let hash = Hash::from_str(
        "0xe4f7aa19a76fcf31a6adff3b400300849e39dd84076765fb3af09d05ee9d787a",
    )
    .unwrap();
    let block_by_hash = prov.get_block_by_hash(hash).await.unwrap();
    assert!(hash == block_by_hash.hash.unwrap());
}

#[tokio::test]
async fn test_trace_block_by_hash() {
    let prov = get_provider();

    let hash = Hash::from_str(
        "0xe2d191e9f663a3a950519eadeadbd614965b694a65a318a0b8f053f2d14261ff",
    )
    .unwrap();
    let trace_by_hash = prov.trace_block_by_hash(hash).await.unwrap();
    // Since we called in the test block the same transaction twice the len
    // should be the same and != 0.
    assert!(
        trace_by_hash[0].struct_logs.len()
            == trace_by_hash[1].struct_logs.len()
    );
    assert!(!trace_by_hash[0].struct_logs.is_empty());
    assert_eq!(trace_by_hash[0].struct_logs.len(), 116);
    assert_eq!(
        trace_by_hash[0].struct_logs.last().unwrap().pc,
        ProgramCounter::from(180)
    );
}

#[tokio::test]
async fn test_get_contract_code() {
    let prov = get_provider();
    let contract_address =
        address!("0xd5f110b3e81de87f22fa8c5e668a5fc541c54e3d");
    let contract_code = get_contract_vec_u8();
    let gotten_contract_code = prov
        .get_code_by_address(contract_address, BlockNumber::Latest)
        .await
        .unwrap();
    assert_eq!(contract_code, gotten_contract_code);
}

#[tokio::test]
async fn test_trace_block_by_number() {
    let prov = get_provider();
    let trace_by_hash = prov.trace_block_by_number(5.into()).await.unwrap();
    // Since we called in the test block the same transaction twice the len
    // should be the same and != 0.
    assert!(
        trace_by_hash[0].struct_logs.len()
            == trace_by_hash[1].struct_logs.len()
    );
    assert!(!trace_by_hash[0].struct_logs.is_empty());
    assert_eq!(trace_by_hash[0].struct_logs.len(), 116);
    assert_eq!(
        trace_by_hash[0].struct_logs.last().unwrap().pc,
        ProgramCounter::from(180)
    );
}

#[tokio::test]
async fn test_get_proof() {
    let prov = get_provider();

    let address =
        Address::from_str("0x7F0d15C7FAae65896648C8273B6d7E43f58Fa842")
            .unwrap();
    let keys = vec![Word::from_str(
        "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
    )
    .unwrap()];
    let proof = prov
        .get_proof(address, keys, BlockNumber::Latest)
        .await
        .unwrap();
    const TARGET_PROOF: &str = r#"{
        "address": "0x7f0d15c7faae65896648c8273b6d7e43f58fa842",
        "accountProof": [
            "0xf873a12050fb4d3174ec89ef969c09fd4391602169760fb005ad516f5d172cbffb80e955b84ff84d8089056bc75e2d63100000a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a0c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
        ],
        "balance": "0x0",
        "codeHash": "0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470",
        "nonce": "0x0",
        "storageHash": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
        "storageProof": [
            {
                "key": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
                "value": "0x0",
                "proof": []
            }
        ]
    }"#;
    assert!(
        serde_json::from_str::<EIP1186ProofResponse>(TARGET_PROOF).unwrap()
            == proof
    );
}

fn get_contract_vec_u8() -> Vec<u8> {
    vec![
        96, 128, 96, 64, 82, 52, 128, 21, 97, 0, 16, 87, 96, 0, 128, 253, 91,
        80, 96, 4, 54, 16, 97, 0, 76, 87, 96, 0, 53, 96, 224, 28, 128, 99, 33,
        132, 140, 70, 20, 97, 0, 81, 87, 128, 99, 46, 100, 206, 193, 20, 97, 0,
        109, 87, 128, 99, 176, 242, 183, 42, 20, 97, 0, 139, 87, 128, 99, 243,
        65, 118, 115, 20, 97, 0, 167, 87, 91, 96, 0, 128, 253, 91, 97, 0, 107,
        96, 4, 128, 54, 3, 129, 1, 144, 97, 0, 102, 145, 144, 97, 1, 60, 86,
        91, 97, 0, 197, 86, 91, 0, 91, 97, 0, 117, 97, 0, 218, 86, 91, 96, 64,
        81, 97, 0, 130, 145, 144, 97, 1, 120, 86, 91, 96, 64, 81, 128, 145, 3,
        144, 243, 91, 97, 0, 165, 96, 4, 128, 54, 3, 129, 1, 144, 97, 0, 160,
        145, 144, 97, 1, 60, 86, 91, 97, 0, 227, 86, 91, 0, 91, 97, 0, 175, 97,
        0, 237, 86, 91, 96, 64, 81, 97, 0, 188, 145, 144, 97, 1, 120, 86, 91,
        96, 64, 81, 128, 145, 3, 144, 243, 91, 128, 96, 0, 129, 144, 85, 80,
        96, 0, 97, 0, 215, 87, 96, 0, 128, 253, 91, 80, 86, 91, 96, 0, 128, 84,
        144, 80, 144, 86, 91, 128, 96, 0, 129, 144, 85, 80, 80, 86, 91, 96, 0,
        128, 97, 0, 249, 87, 96, 0, 128, 253, 91, 96, 0, 84, 144, 80, 144, 86,
        91, 96, 0, 128, 253, 91, 96, 0, 129, 144, 80, 145, 144, 80, 86, 91, 97,
        1, 25, 129, 97, 1, 6, 86, 91, 129, 20, 97, 1, 36, 87, 96, 0, 128, 253,
        91, 80, 86, 91, 96, 0, 129, 53, 144, 80, 97, 1, 54, 129, 97, 1, 16, 86,
        91, 146, 145, 80, 80, 86, 91, 96, 0, 96, 32, 130, 132, 3, 18, 21, 97,
        1, 82, 87, 97, 1, 81, 97, 1, 1, 86, 91, 91, 96, 0, 97, 1, 96, 132, 130,
        133, 1, 97, 1, 39, 86, 91, 145, 80, 80, 146, 145, 80, 80, 86, 91, 97,
        1, 114, 129, 97, 1, 6, 86, 91, 130, 82, 80, 80, 86, 91, 96, 0, 96, 32,
        130, 1, 144, 80, 97, 1, 141, 96, 0, 131, 1, 132, 97, 1, 105, 86, 91,
        146, 145, 80, 80, 86, 254, 162, 100, 105, 112, 102, 115, 88, 34, 18,
        32, 198, 65, 17, 183, 105, 192, 24, 239, 185, 163, 114, 200, 208, 240,
        163, 224, 232, 124, 166, 82, 153, 136, 202, 171, 161, 44, 117, 159, 44,
        234, 223, 52, 100, 115, 111, 108, 99, 67, 0, 8, 10, 0, 51,
    ]
}
