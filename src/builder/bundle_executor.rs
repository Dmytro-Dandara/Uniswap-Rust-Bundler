use anyhow::Result;
use crossterm::style::Stylize;
use ethers::prelude::k256::ecdsa::SigningKey;
use ethers::prelude::{Http, LocalWallet, Middleware, Provider, Wallet};
use ethers_flashbots::{BundleRequest, FlashbotsMiddleware};
use futures::future::join_all;
use std::sync::Arc;
use tracing::{error, info, warn};
use url::Url;

use crate::types::settings::Settings;
use crate::types::utils::{self, enter_to_proceed};

pub async fn execute(
    config: Settings,
    provider: Arc<Provider<Http>>,
    bundle_signer: Arc<Wallet<SigningKey>>,
    bundle: BundleRequest,
) -> Result<()> {
    let local_wallet: LocalWallet = bundle_signer.as_ref().clone();
    let fb_client = FlashbotsMiddleware::new(
        provider.clone(),
        Url::parse(&config.connection.flashbots_url)?,
        local_wallet,
    );

    // Simulate bundle.
    let retries = config.bundle.retries;
    let block_number = fb_client.get_block_number().await?;
    let bundle_sim = bundle
        .clone()
        .set_block(block_number + 1)
        .set_simulation_block(block_number)
        .set_simulation_timestamp(0);

    let simulated_bundle = fb_client.simulate_bundle(&bundle_sim).await;
    match simulated_bundle {
        Ok(simulated_bundle_res) => {
            info!("Simulated bundle: {:#?}", simulated_bundle_res);
            let txs = simulated_bundle_res.transactions.clone();
            for tx in txs {
                info!("Tx hash: {:#?}", tx);
                let mut err = false;
                if let Some(error) = tx.error {
                    error!("Tx error: {:#?}", error);
                    err = true;
                }
                if err {
                    error!("Simulation failed. Exiting.");
                    std::process::exit(1);
                }
            }
        }
        Err(send_error) => {
            let error_message = send_error.to_string();
            error!("Error simulating bundle: {}", error_message);
            if let Some(msg) = utils::parse_insufficient_funds_message(&error_message) {
                error!("{}", msg);
            }
            std::process::exit(1);
        }
    }

    info!("{}", "Simulation Ok. Send it? (Enter)".white());
    enter_to_proceed();
    // Send bundle.
    let block_number = fb_client.get_block_number().await?;

    let bundles = (1..retries)
        .map(|i| bundle.clone().set_block(block_number + i))
        .collect::<Vec<BundleRequest>>();

    let send_all = bundles.iter().map(|bundle| fb_client.send_bundle(&bundle));
    let results = join_all(send_all).await;
    for result in results {
        match result {
            Ok(pending_bundle_res) => match pending_bundle_res.await {
                Ok(res) => {
                    info!("Bundle mined with hash: {:#?}", res);
                    break;
                }
                Err(err) => {
                    warn!("Bundle not mined: {:#?}", err);
                }
            },
            Err(send_error) => {
                warn!("Error sending bundle: {}", send_error.to_string());
            }
        }
    }
    Ok(())
}
