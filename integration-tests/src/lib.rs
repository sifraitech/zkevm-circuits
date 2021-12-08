use bus_mapping::rpc::GethClient;
use ethers_core::k256::ecdsa::SigningKey;
use ethers_providers::{Http, Provider};
use ethers_signers::{coins_bip39::English, MnemonicBuilder, Wallet};
use lazy_static::lazy_static;
use std::env::{self, VarError};
use std::time::Duration;
use url::Url;

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

pub fn get_client() -> GethClient<Http> {
    let transport = Http::new(Url::parse(&GETH0_URL).unwrap());
    GethClient::new(transport)
}

pub fn get_provider() -> Provider<Http> {
    let transport = Http::new(Url::parse(&GETH0_URL).unwrap());
    Provider::new(transport).interval(Duration::from_millis(100))
}

const PHRASE: &str = "work man father plunge mystery proud hollow address reunion sauce theory bonus";

pub fn get_wallet(index: u32) -> Wallet<SigningKey> {
    // Access mnemonic phrase.
    // Child key at derivation path: m/44'/60'/0'/0/{index}
    MnemonicBuilder::<English>::default()
        .phrase(PHRASE)
        .index(index)
        .unwrap()
        .build()
        .unwrap()
}
