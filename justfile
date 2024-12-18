check:
    cargo +stable fmt --all -- --check
    cargo +stable clippy --all-targets -- -D warnings

download-deps:
    mkdir -p artifacts target
    wget "https://github.com/CosmWasm/cw-plus/releases/latest/download/cw20_base.wasm" -O artifacts/cw20_base.wasm
    wget "https://github.com/CosmWasm/cw-plus/releases/latest/download/cw4_group.wasm" -O artifacts/cw4_group.wasm
    wget "https://github.com/CosmWasm/cw-nfts/releases/latest/download/cw721_base.wasm" -O artifacts/cw721_base.wasm
    wget "https://github.com/DA0-DA0/dao-contracts/releases/latest/download/dao_dao_core.wasm" -O artifacts/dao_dao_core.wasm
    wget "https://github.com/DA0-DA0/dao-contracts/releases/latest/download/dao_proposal_sudo.wasm" -O artifacts/dao_proposal_sudo.wasm
    wget "https://github.com/DA0-DA0/dao-contracts/releases/latest/download/dao_proposal_single.wasm" -O artifacts/dao_proposal_single.wasm

test:
    cargo test

ci:
    just check
    just test

udeps:
    cargo +nightly udeps

optimize:
    docker run --rm \
        -v "$(pwd)":/code \
        --mount type=volume,source=arena_cache,target=/code/target \
        --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
        "$DOCKER_OPTIMIZER"

schema:
    ./scripts/schema.sh

deploy network target:
    cargo run --bin scripts -- deploy {{network}} {{target}}

help:
    @echo "Available tasks:"
    @echo "  check          Run formatting and lint checks"
    @echo "  download-deps  Download dependency WASM files"
    @echo "  test           Run tests"
    @echo "  udeps          Check for unused dependencies"
    @echo "  optimize       Optimize WASM files using Docker"
    @echo "  schema         Generate JSON schemas"
    @echo "  deploy         Deploy contracts"
    @echo "                 usage: just deploy <network> <target>"
    @echo "                 network: testnet or mainnet"
    @echo "                 target: all or specific contract name"
