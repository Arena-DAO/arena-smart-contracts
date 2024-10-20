check:
    cargo +stable fmt --all -- --check
    cargo +stable clippy --all-targets -- -D warnings

download-deps:
    mkdir -p artifacts target
    wget https://github.com/CosmWasm/cw-plus/releases/latest/download/cw20_base.wasm -O artifacts/cw20_base.wasm
    wget https://github.com/CosmWasm/cw-plus/releases/latest/download/cw4_group.wasm -O artifacts/cw4_group.wasm
    wget https://github.com/CosmWasm/cw-nfts/releases/latest/download/cw721_base.wasm -O artifacts/cw721_base.wasm
    wget https://github.com/DA0-DA0/dao-contracts/releases/latest/download/dao_dao_core.wasm -O artifacts/dao_dao_core.wasm
    wget https://github.com/DA0-DA0/dao-contracts/releases/latest/download/dao_proposal_sudo.wasm -O artifacts/dao_proposal_sudo.wasm
    wget https://github.com/DA0-DA0/dao-contracts/releases/latest/download/dao_proposal_single.wasm -O artifacts/dao_proposal_single.wasm

test:
    cargo test

udeps:
    cargo +nightly udeps

optimize:
    docker run --rm \
        -v "$(pwd)":/code \
        --mount type=volume,source=arena_cache,target=/code/target \
        --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
        cosmwasm/optimizer:0.16.1

schema:
    ./scripts/schema.sh

deploy_testnet:
    cargo run --bin scripts -- deploy testnet all

deploy_mainnet:
    cargo run --bin scripts -- deploy mainnet all