# Arena DAO Smart Contracts

[![Basic](https://github.com/Arena-DAO/arena-smart-contracts/actions/workflows/basic.yml/badge.svg)](https://github.com/Arena-DAO/arena-smart-contracts/actions/workflows/basic.yml)

This project contains the smart contracts for the Arena DAO. The contracts are written in Rust and use the CosmWasm framework. The project also includes TypeScript code for generating TypeScript types from the contract schemas.

## Project Structure

The project is organized into several directories:

- `contracts`: This directory contains the smart contract code. Each contract is in its own subdirectory. Each contract has a Cargo.toml file that specifies its dependencies and a src directory that contains the Rust source code.

- `packages`: This directory contains Rust packages that are used by the contracts. Each package has a Cargo.toml file and a src directory.

- `scripts`: This directory contains batch files for generating the contract schemas and TypeScript types.

- `ts-codegen`: This directory contains TypeScript code for generating TypeScript types from the contract schemas. The src directory contains the source code and the package.json file specifies the dependencies.

## Building and Testing

The `scripts/check.bat` file is used to check for unused dependencies, format the code, and run clippy for linting. It uses the nightly version of cargo for these tasks.

The `scripts/gen.bat` file is used to generate the contract schemas and TypeScript types. It first generates the schemas for all contracts and then generates the TypeScript types.

The `scripts/optimize.bat` file is used to optimize the smart contracts. It uses the Docker image `cosmwasm/workspace-optimizer:0.14.0` to run the optimization process.

The GitHub Actions workflows in the `.github/workflows` directory are used to run tests and lints on every push and pull request.

## TypeScript Code Generation

The TypeScript code generation is done by the `ts-codegen` package. The `src/codegen.ts` file uses the `@cosmwasm/ts-codegen` package to generate TypeScript types from the contract schemas. The generated types are output to the `./output` directory.

## Smart Contracts

The smart contracts are written in Rust and use the CosmWasm framework. Each contract has its own directory under the `contracts` directory. The `Cargo.toml` file in each contract directory specifies the contract's dependencies.

## Continuous Integration

The project uses GitHub Actions for continuous integration. The workflows are defined in the `.github/workflows` directory. The `basic.yml` workflow runs tests and lints on every push and pull request.

## Architecture Diagram

![image](https://showme.redstarplugin.com/d/d:LvtiZJV2)
