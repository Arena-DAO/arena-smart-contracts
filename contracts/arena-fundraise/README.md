# Arena Fundraise Contract

This contract is designed to facilitate fundraising for native tokens.

## Contract Usage

### Instantiation

The contract is instantiated with the following parameters:

- `fundraise`: The coin containing a denom and amount to be distributed on a successful fundraiser.
- `deposit_denom`: The denomination of the deposits.
- `soft_cap`: The minimum amount that needs to be raised for the fundraise to be considered successful.
- `hard_cap`: (Optional) The maximum amount that can be raised.
- `start`: (Optional) The start time of the fundraise campaign.
- `duration`: The duration of the fundraise campaign.

### Execute Messages

- `Deposit`: Allows a user to deposit funds into the fundraise campaign.
- `Withdraw`: Allows a user to withdraw their deposit or reward based off the current fundraise status. Users can withdraw deposits at any point while active. They withdraw deposits on failure and rewards on success.
- `Expire`: Locks in the fundraiser's status.

### Query Messages

- `Config`: Returns the configuration of the fundraise campaign.
- `TotalDeposited`: Returns the total amount deposited into the fundraise campaign.
- `Deposit`: Returns the amount deposited by a specific address.
- `Reward`: Returns the reward for a specific address.
- `DumpState`: Returns the complete state of the fundraise campaign for a specific address.

## Error Handling

The contract includes error handling for various scenarios such as invalid amounts, expired start times, and incorrect cap configurations.
