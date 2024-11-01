// #![feature(rustc_private)]
// #![feature(async_closure)]
// #![feature(let_chains)]

use crossterm::style::Stylize;
use ethers::providers::{Http, Middleware, Provider};
use ethers::signers::{LocalWallet, Signer};
use gravity_bundler::builder::{bundle_builder, bundle_executor};
use gravity_bundler::types::settings::Settings;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO) // Set maximum log level to INFO
        .init();
    info!("{}", "Starting the bundler".yellow());

    // Use read_config for creating and reading the settings from the config.json file
    // let settings = Settings::read_config("config.json")?;

    // Use new() for reading the settings from config.toml file
    let settings = Settings::new()?;
    info!("{:#?}", settings);

    let provider = Arc::new(Provider::<Http>::try_from(
        settings.connection.ethereum_rpc_url.as_str(),
    )?);

    // Read the main wallet's private key
    // let main_wallet: LocalWallet = settings.sniper.private_keys[0].parse()?;
    // let signer = Arc::new(main_wallet.with_chain_id(provider.get_chainid().await?.as_u64()));

    let chain_id = provider.get_chainid().await.unwrap().as_u64();
    info!("Connected to the chain with ID: {}", chain_id);

    let signers = settings
        .sniper
        .private_keys
        .iter()
        .map(|key| {
            let wallet: LocalWallet = key.parse().unwrap();
            let wallet = wallet.with_chain_id(chain_id);
            Arc::new(wallet)
        })
        .collect::<Vec<_>>();

    let bundle_signer = Arc::new(
        settings
            .bundle
            .bundler_key
            .parse::<LocalWallet>()?
            .with_chain_id(chain_id),
    );

    // Prepare the bundle
    let bundle =
        bundle_builder::build_txs(&settings, provider.clone(), signers, bundle_signer.clone())
            .await?;
    info!("Bundle: {:#?}", bundle);

    // Execute the bundle
    bundle_executor::execute(settings, provider.clone(), bundle_signer, bundle).await?;

    Ok(())
}
