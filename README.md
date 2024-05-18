# Arena DAO Smart Contracts

[![Basic](https://github.com/Arena-DAO/arena-smart-contracts/actions/workflows/basic.yml/badge.svg)](https://github.com/Arena-DAO/arena-smart-contracts/actions/workflows/basic.yml)

This project contains the smart contracts for the Arena DAO. The contracts are written in Rust and use the CosmWasm framework. The project also includes TypeScript code for generating TypeScript types from the contract schemas.

## License

This project is licensed under the terms of the GNU General Public License v3.0. See the [LICENSE](./LICENSE) file for details.

## Project Structure

The project is organized into several directories:

- `contracts`: This directory contains the smart contract code. Each contract is in its own subdirectory. Each contract has a Cargo.toml file that specifies its dependencies and a src directory that contains the Rust source code.

- `packages`: This directory contains Rust packages that are used by the contracts. Each package has a Cargo.toml file and a src directory.

- `scripts`: This directory contains batch files for checking and optimizing the contracts.


## Building and Testing

The `scripts/check` file is used to check for unused dependencies, format the code, and run clippy for linting. It uses the nightly version of cargo for these tasks.

The `scripts/gen` file is used to generate the contract schemas and TypeScript types. It first generates the schemas for all contracts and then generates the TypeScript types.

The `scripts/optimize` file is used to optimize the smart contracts. It uses the Docker image `cosmwasm/workspace-optimizer:0.15.1` to run the optimization process.

The GitHub Actions workflows in the `.github/workflows` directory are used to run tests and lints on every push and pull request.

## Smart Contracts

The smart contracts are written in Rust and use the CosmWasm framework. Each contract has its own directory under the `contracts` directory. The `Cargo.toml` file in each contract directory specifies the contract's dependencies.

## Continuous Integration

The project uses GitHub Actions for continuous integration. The workflows are defined in the `.github/workflows` directory. The `basic.yml` workflow runs tests and lints on every push and pull request.

## Architecture Diagram

![image](https://showme.redstarplugin.com/d/d:LvtiZJV2)
