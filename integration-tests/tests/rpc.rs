#![cfg(feature = "test_rpc")]

use ethers_providers::Http;
use url::Url;
use bus_mapping::eth_types::{Hash, Address, Word, EIP1186ProofResponse};
use bus_mapping::rpc::{GethClient, BlockNumber};
use std::str::FromStr;
use std::env::{self, VarError};
use integration_tests::GETH0_URL;

#[tokio::test]
async fn test_get_block_by_hash() {
    let transport = Http::new(Url::parse(&GETH0_URL).unwrap());

    let hash = Hash::from_str("0xe4f7aa19a76fcf31a6adff3b400300849e39dd84076765fb3af09d05ee9d787a").unwrap();
    let prov = GethClient::new(transport);
    let block_by_hash = prov.get_block_by_hash(hash).await.unwrap();
    assert!(hash == block_by_hash.hash.unwrap());
}

#[tokio::test]
async fn test_get_block_by_number() {
    let transport = Http::new(Url::parse(&GETH0_URL).unwrap());

    let hash = Hash::from_str("0xe4f7aa19a76fcf31a6adff3b400300849e39dd84076765fb3af09d05ee9d787a").unwrap();
    let prov = GethClient::new(transport);
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
async fn test_trace_block_by_hash() {
    let transport = Http::new(Url::parse(&GETH0_URL).unwrap());

    let hash = Hash::from_str("0xe2d191e9f663a3a950519eadeadbd614965b694a65a318a0b8f053f2d14261ff").unwrap();
    let prov = GethClient::new(transport);
    let trace_by_hash = prov.trace_block_by_hash(hash).await.unwrap();
    // Since we called in the test block the same transaction twice the len
    // should be the same and != 0.
    assert!(
        trace_by_hash[0].struct_logs.len()
            == trace_by_hash[1].struct_logs.len()
    );
    assert!(!trace_by_hash[0].struct_logs.is_empty());
}

#[tokio::test]
async fn test_trace_block_by_number() {
    let transport = Http::new(Url::parse(&GETH0_URL).unwrap());
    let prov = GethClient::new(transport);
    let trace_by_hash = prov.trace_block_by_number(5.into()).await.unwrap();
    // Since we called in the test block the same transaction twice the len
    // should be the same and != 0.
    assert!(
        trace_by_hash[0].struct_logs.len()
            == trace_by_hash[1].struct_logs.len()
    );
    assert!(!trace_by_hash[0].struct_logs.is_empty());
}

#[tokio::test]
async fn test_get_proof() {
    let transport = Http::new(Url::parse(&GETH0_URL).unwrap());
    let prov = GethClient::new(transport);

    let address =
        Address::from_str("0x7F0d15C7FAae65896648C8273B6d7E43f58Fa842")
            .unwrap();
    let keys = vec![Word::from_str("0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421").unwrap()];
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
