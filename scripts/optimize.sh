docker run --rm \
  -v "$(pwd)":/code \
  --mount type=volume,source=arena_cache,target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/optimizer:0.15.1