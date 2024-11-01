# Summary

The bot opens trading, deploys liquidity and buys back tokens all in one flashbots bundle

`config.toml` file is used to configure the bot.

1. Deploy the contract and set `[contract]` `address`.
2. Set a list of private keys what will buy the tokens under `[sniper]` `private_keys`, the FIRST wallet is the contract owner and the owner of the liquidity.
3. Send ETH and Token to the contract that should create the LP on Uniswap - they should rest on the contract balance and they will be added to the liquidity pair in full.
4. Set up the percent of totalsupply to buy on each wallet in the `buyback` param under `[sniper]`.
5. Set up the `gas_price` and `gas_limit` in the `[sniper]` section.
6. Run the bot with `cargo run --release`

## Deploying with forge

1. deploying

```shell
forge create src/milk.sol:milk --private-key= --rpc-url https://sepolia.infura.io/v3/1e18b754a2b1477694d21fd833aaf203
```

2. sending 400B tokens

```shell
cast send  0x845Fc224b66bb8704B9E9AbD47A571e5dE183DB3 "transfer(address,uint256)"  0x845Fc224b66bb8704B9E9AbD47A571e5dE183DB3 400000000000000000000 --private-key= --rpc-url https://sepolia.infura.io/v3/1e18b754a2b1477694d21fd833aaf203
```

3. sending 1.0 ETH

```shell
cast send  0x845Fc224b66bb8704B9E9AbD47A571e5dE183DB3 --value 1000000000000000000 --private-key  --rpc-url https://sepolia.infura.io/v3/1e18b754a2b1477694d21fd833aaf203
```
# Uniswap-rust-bundler
