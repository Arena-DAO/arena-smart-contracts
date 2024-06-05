# Arena DAO Smart Contracts

[![Basic](https://github.com/Arena-DAO/arena-smart-contracts/actions/workflows/basic.yml/badge.svg)](https://github.com/Arena-DAO/arena-smart-contracts/actions/workflows/basic.yml)

This project contains the smart contracts for the Arena DAO. The contracts are written in Rust and use the CosmWasm framework. The project also includes TypeScript code for generating TypeScript types from the contract schemas.

## License

This project is licensed under the terms of the GNU General Public License v3.0. See the [LICENSE](./LICENSE) file for details.

## Project Structure

The project is organized into several directories:

- `contracts`: This directory contains the smart contract code. Each contract is in its own subdirectory. Each contract has a Cargo.toml file that specifies its dependencies and a src directory that contains the Rust source code.

- `packages`: This directory contains Rust packages that are used by the contracts. Each package has a Cargo.toml file and a src directory.

- `scripts`: This directory contains the justfile and `cw-orch` scripts and tests.


## Scripts

This project uses `just` as a command runner to simplify common tasks. Below are the available scripts:

### `check`
Runs formatting and linting checks to ensure code quality:

- **Format check**: Ensures the code formatting adheres to the standard Rust format using `cargo fmt`.
- **Lint check**: Runs `clippy` to catch common mistakes and improve your Rust code.

### `download-deps`

Downloads necessary wasm artifacts for the project:

Downloads various wasm modules like cw20_base, cw4_group, cw721_base, from the CosmWasm repository and dao_contracts from the DAO-DAO repository into the artifacts directory.

### `udeps`

Finds unused dependencies in the project using cargo udeps. This helps in identifying unnecessary dependencies that can be removed to streamline the project.

### `optimize`

Optimizes the wasm builds using Docker and the CosmWasm optimizer.

### `schema`

Generates JSON schema for messages used in the smart contracts.

## Continuous Integration

The project uses GitHub Actions for continuous integration. The workflows are defined in the `.github/workflows` directory. The `basic.yml` workflow runs tests and lints on every push and pull request.