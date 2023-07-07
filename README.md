# kudos
Kudos contract for NDC

## Setup [Testnet]

1. Build conrtact
```
RUSTFLAGS='-C link-arg=-s' cargo build --all --target wasm32-unknown-unknown --release
```
2. Init contract
```
near call $CONRTACT_ID init '{"iah_registry": "registry-unstable.i-am-human.testnet"}' --accountId rubycoptest.testnet
near call $CONRTACT_ID set_external_db '{"external_db_id": "v1.social08.testnet"}' --accountId rubycoptest.testnet --amount 5
```
3. Deploy it on testnet
```
near dev-deploy target/wasm32-unknown-unknown/release/kudos_contract.wasm
```