# Discord Identity Contract

A CosmWasm smart contract that manages Discord user identities and their associated blockchain addresses. The contract includes a faucet feature that sends tokens to new users upon registration.

## Features

- Links Discord user IDs with blockchain addresses
- Maintains a bidirectional mapping between Discord IDs and addresses
- Includes a configurable faucet that sends tokens to newly registered users
- Implements ownership controls for administrative functions

## Key Functions

### Administration

- **Set Faucet Amount**: Owners can configure the amount of tokens sent to new users
- **Withdraw**: Owners can withdraw all funds from the contract
- **Update Ownership**: Transfer or renounce contract ownership

### User Management

- **Set Profile**: Associates a Discord user ID with a blockchain address
- **Query User ID**: Retrieve the Discord user ID associated with an address