use anyhow::Result;
use ethers::abi::Abi;
use ethers::addressbook::Address;
use ethers::contract::Contract;
use ethers::prelude::k256::ecdsa::SigningKey;
use ethers::prelude::*;
use ethers::prelude::{Http, Middleware, Provider, Signer};
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers_flashbots::BundleRequest;
use rand::Rng;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use thousands::Separable;
use tracing::{error, info};

use crate::types::settings::Settings;

pub fn deadline_timestamp() -> u64 {
    let deadline = SystemTime::now() + Duration::from_secs(60 * 1); // 3 minutes from now
    deadline
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub async fn build_txs(
    settings: &Settings,
    provider: Arc<Provider<Http>>,
    signers: Vec<Arc<Wallet<SigningKey>>>,
    bundle_signer: Arc<Wallet<SigningKey>>,
) -> Result<BundleRequest> {
    let chain_id = U64::from(provider.get_chainid().await?.low_u64());
    let mut bundle = BundleRequest::new();

    let uniswap_router_address = settings.contract.uniswap_v2_router.parse::<Address>()?;
    let uniswap_router_abi: Abi = serde_json::from_str(include_str!("uniswap_v2_router_abi.json"))?;
    let uniswap_router =
        Contract::new(uniswap_router_address, uniswap_router_abi, provider.clone());

    let contract_address = settings.contract.address.parse::<Address>()?;
    let contract_abi: Abi = serde_json::from_str(include_str!("abi.json"))?;
    let contract = Contract::new(contract_address, contract_abi, provider.clone());

    let base_buyback = settings.sniper.buyback;
    let base_buyback_max = settings.sniper.max_limit;

    // Define the range for deviation
    let min_buyback = base_buyback * 0.98; // 2% below base buyback
    let max_buyback = base_buyback_max; // 2% above base buyback

    // Initialize the random number generator
    let mut rng = rand::thread_rng();

    let owner = contract.method::<(), Address>("owner", ())?.call().await?;
    info!("Contract owner: {:?}", owner);
    if owner != signers[0].address() {
        error!("The contract owner is not the same as the first signer. Exiting.");
        std::process::exit(1);
    } else {
        info!("The contract owner is the same as the first signer, confirmed.");
    }

    let total_supply = contract
        .method::<(), U256>("totalSupply", ())?
        .call()
        .await?;
    let decimals = contract.method::<(), U256>("decimals", ())?.call().await?;
    info!(
        "Total supply: {}",
        (total_supply / 10_u64.pow(decimals.as_u64() as u32)).separate_with_spaces()
    );
    let amount_per_wallet_estimate = (total_supply.as_u128() as f64 * max_buyback) as u128;

    info!(
        "Wallets for buyback: {}, buying {:.2}% of total supply per each wallet which is {} tokens",
        (signers.len() - 1).to_string(),
        max_buyback * 100.0,
        (amount_per_wallet_estimate / 10_u128.pow(decimals.as_u64() as u32)).separate_with_spaces()
    );

    info!(
        "Total buyback: {} tokens, {}%",
        (amount_per_wallet_estimate * (signers.len() as u128 - 1)
            / 10_u128.pow(decimals.as_u64() as u32))
        .separate_with_spaces(),
        max_buyback * 100.0 * (signers.len() - 1) as f64
    );
    // 1. estimate the amount of Eth that should exist on every wallet
    let reserve_eth: U256 = provider.get_balance(contract.address(), None).await?;

    let reserve_token: U256 = contract
        .method::<Address, U256>("balanceOf", contract.address())?
        .call()
        .await?;

    // use 77% tokens from the reserve_token
    let reserve_token = reserve_token * U256::from(77) / U256::from(100);

    // hard-coded
    // let reserve_token = U256::from(308000000000000000000_u128);

    if reserve_eth == U256::zero() || reserve_token == U256::zero() {
        error!("The contract should have both ETH and tokens in the reserve. Exiting.");
        std::process::exit(1);
    };

    info!(
        "Liquidity pair will be funded with \
    reserve_eth: {},\
    reserve_token: {}",
        reserve_eth.as_u64() as f64 / 10_f64.powf(18_f64),
        (reserve_token / 10_u64.pow(decimals.as_u64() as u32)).separate_with_spaces()
    );

    let block = provider
        .get_block(BlockId::Number(BlockNumber::Latest))
        .await?
        .expect("block not found");

    let base_fee = block.base_fee_per_gas.expect("base fee not available");
    info!("Base fee: {}", base_fee.as_u64());
    let priority_fee = U256::from((settings.bundle.priority_fee * 10_f64.powf(18_f64)) as u128);
    let max_fee_per_gas = base_fee + priority_fee;
    info!("Max fee per gas: {}", max_fee_per_gas.as_u64());
    // Create the openTrading transaction
    let mut nonce = provider
        .get_transaction_count(signers[0].address(), None)
        .await?;

    let tx_request = contract
        .method::<_, ()>("openTrading", ())?
        .gas(U256::from(3_000_000))
        .from(signers[0].address())
        .gas_price(max_fee_per_gas)
        .nonce(nonce);

    let mut tx = tx_request.tx;
    tx.set_chain_id(chain_id);
    let signature = signers[0].sign_transaction(&tx).await?;
    bundle.add_transaction(tx.rlp_signed(&signature));

    let mut current_reserve_eth = reserve_eth;
    let mut current_reserve_token = reserve_token;
    let mut err = false;

    for wallet in signers.iter().skip(1) {
        let random_buyback = rng.gen_range(min_buyback..max_buyback);
        let amount_per_wallet = (total_supply.as_u128() as f64 * random_buyback) as u128;
        let amount_per_wallet_u256 = U256::from(amount_per_wallet);
        let path = vec![
            ethers::types::Address::from_str(&settings.contract.weth).unwrap(),
            contract_address,
        ];

        let get_output_tokens_method = uniswap_router
            .method::<_, U256>(
                "getAmountIn",
                (
                    amount_per_wallet_u256,
                    current_reserve_eth,
                    current_reserve_token,
                ),
            )
            .expect("Failed to get getAmountsOut method");

        let get_output_tokens_result = get_output_tokens_method.call().await;

        let eth_input = get_output_tokens_result.expect("Failed to get getAmountsOut result");

        current_reserve_eth += eth_input;
        current_reserve_token -= amount_per_wallet_u256;

        println!(
        "\nEmulated swap to get {:.7} tokens:\n amount to spend: {:.7} ETH,\n after swap: reserve_eth: {:.7} ETH,\n reserve_token: {:.7} tokens\n",
        amount_per_wallet_u256.as_u128() as f64 / 10_f64.powf(decimals.as_u32() as f64),
        eth_input.as_u128() as f64 / 10_f64.powf(18_f64),
        current_reserve_eth.as_u128() as f64 / 10_f64.powf(18_f64),
        current_reserve_token.as_u128() as f64 / 10_f64.powf(decimals.as_u32() as f64)
    );

        // Check if the wallet has enough ETH to cover the swap
        let wallet_balance = provider.get_balance(wallet.address(), None).await?;
        if wallet_balance < eth_input {
            err = true;
            error!(
                "Not enough ETH for {:?}. Required: {:.7}, available: {:.7}",
                wallet.address(),
                eth_input.as_u64() as f64 / 10_f64.powf(18_f64),
                wallet_balance.as_u64() as f64 / 10_f64.powf(18_f64)
            );
            continue;
        } else {
            info!(
                "{:?} : ETH required to swap (w/o gas): {:.7}, available: {:.7}",
                wallet.address(),
                eth_input.as_u64() as f64 / 10_f64.powf(18_f64),
                wallet_balance.as_u64() as f64 / 10_f64.powf(18_f64)
            );
        }
        nonce = provider
            .get_transaction_count(wallet.address(), None)
            .await?;
        let tx_request = uniswap_router
            .method::<_, ()>(
                "swapETHForExactTokens",
                (
                    amount_per_wallet_u256,
                    path,
                    wallet.address(),
                    U256::from(2_000_000_000_000_000_000_u64),
                ),
            )?
            .value(eth_input) // Maximum amount of ETH you are willing to spend
            .gas(U256::from(300_000))
            .gas_price(max_fee_per_gas)
            .from(wallet.address())
            .nonce(nonce);

        let mut tx = tx_request.tx;
        tx.set_chain_id(chain_id);
        // Sign the transaction using the wallet
        let signature = wallet.sign_transaction(&tx).await?;
        bundle.add_transaction(tx.rlp_signed(&signature));
    }
    if err {
        error!("Some wallets do not have enough ETH to cover the swaps. Exiting.");
        std::process::exit(1);
    }

    // Adding a tip to the miner
    let miner_tip = U256::from((settings.bundle.miner_tip * 10_f64.powf(18_f64)) as u128);
    let miner_tip_tx: TypedTransaction = TransactionRequest::new()
        .to("0x0000000000000000000000000000000000000000".parse::<Address>()?) // block.coinbase address
        .value(miner_tip)
        .gas_price(max_fee_per_gas)
        .gas(21000)
        .from(bundle_signer.address())
        .nonce(
            provider
                .get_transaction_count(bundle_signer.address(), None)
                .await?,
        )
        .into();

    let signed_miner_tip_tx = bundle_signer.sign_transaction(&miner_tip_tx).await?;
    bundle.add_transaction(miner_tip_tx.rlp_signed(&signed_miner_tip_tx));

    // info!("{}","Press Enter to simulate the bundle".white());
    // crate::utils::enter_to_proceed();
    Ok(bundle)
}
