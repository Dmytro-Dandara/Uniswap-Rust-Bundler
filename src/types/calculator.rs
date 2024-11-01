use ethers::types::U256;

#[derive(Debug)]
pub struct ConstantProduct {}

impl ConstantProduct {
    pub fn calculate_eth_needed(
        token_amount: U256,
        reserve_eth: &mut U256,
        reserve_token: &mut U256,
        token_decimals: U256,
    ) -> U256 {
        let fee_numerator = U256::from(997);
        let fee_denominator = U256::from(1000);
        let amount_with_fee = token_amount
            .checked_mul(fee_numerator)
            .unwrap()
            .checked_div(fee_denominator)
            .unwrap();

        // Calculate how much ETH is needed based on the input amount and current reserves
        let numerator = *reserve_eth * amount_with_fee;
        let denominator = *reserve_token + amount_with_fee; // add amount_with_fee to reserve_token

        let eth_needed = numerator / denominator; // remove the rounding up

        // Update reserves
        *reserve_eth += eth_needed;
        *reserve_token -= token_amount;

        // Logging for debugging purposes
        println!(
            "\nEmulated swap to get {:.7} tokens:\n amount to spend: {:.7} ETH, \nafter swap: reserve_eth: {:.7} ETH,\n reserve_token: {:.7} tokens\n",
            token_amount.as_u128() as f64 / 10_f64.powf(token_decimals.as_u32() as f64),
            eth_needed.as_u128() as f64 / 10_f64.powf(18_f64),
            reserve_eth.as_u128() as f64 / 10_f64.powf(18_f64),
            reserve_token.as_u128() as f64 / 10_f64.powf(token_decimals.as_u32() as f64)
        );

        eth_needed
    }

    // function getAmountIn(uint amountOut, uint reserveIn, uint reserveOut) internal pure returns (uint amountIn) {
    //     require(amountOut > 0, 'UniswapV2Library: INSUFFICIENT_OUTPUT_AMOUNT');
    //     require(reserveIn > 0 && reserveOut > 0, 'UniswapV2Library: INSUFFICIENT_LIQUIDITY');
    //     uint numerator = reserveIn.mul(amountOut).mul(1000);
    //     uint denominator = reserveOut.sub(amountOut).mul(997);
    //     amountIn = (numerator / denominator).add(1);
    // }
}
