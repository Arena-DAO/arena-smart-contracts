# Arena Token Gateway

The `arena-token-gateway` contract manages applications for DAO membership and token distribution. It serves as the central mechanism for onboarding new members and aligning their incentives with the DAO's long-term success.

## How It Works

1. Potential members submit applications using the `Apply` message.
2. DAO members review applications through the DAO's governance process.
3. Accepted applications trigger token distribution with the configured vesting schedule.
4. Rejected applications can be withdrawn or updated for resubmission.

## Contract Messages

### InstantiateMsg

Initializes the Arena Token Gateway contract with the following parameters:

- `owner`: The DAO address
- `config`: Vesting configuration

### ExecuteMsg

The contract supports the following execute messages:

- `Apply`: Submit an application for DAO membership
- `Update`: Update an existing application
- `Withdraw`: Withdraw an application
- `AcceptApplication`: Accept an application and initiate token distribution
- `RejectApplication`: Reject an application with an optional reason
- `UpdateVestingConfiguration`: Modify the vesting configuration
- `UpdateOwnership`: Update the contract ownership (from cw_ownable)

### QueryMsg

The contract supports the following query messages:

- `Ownership`: Get the current contract owner
- `Application`: Get details of a specific application
- `Applications`: List applications with optional filters
- `VestingConfiguration`: Get the current vesting configuration

## Vesting Configuration

The vesting configuration is defined as follows:

```rust
#[cw_serde]
pub struct VestingConfiguration {
    pub upfront_ratio: Decimal,
    pub vesting_time: u64,
    pub denom: String,
    pub cw_vesting_code_id: u64,
}
```

- `upfront_ratio`: The ratio of tokens to be distributed immediately upon acceptance
- `vesting_time`: The duration of the vesting period in seconds
- `denom`: The denomination of the tokens being distributed
- `cw_vesting_code_id`: The code ID of the cw-vesting contract to be used

## Application Structure

Applications are structured as follows:

```rust
#[cw_serde]
pub struct ApplicationInfo {
    pub title: String,
    pub description: String,
    pub requested_amount: Uint128,
    pub project_links: Vec<ProjectLink>,
    pub status: ApplicationStatus,
}

#[cw_serde]
pub enum ApplicationStatus {
    Pending {},
    Accepted {},
    Rejected { reason: Option<String> },
}

#[cw_serde]
pub struct ProjectLink {
    pub title: String,
    pub url: String,
}
```

## Important Notes

1. The DAO must have the vesting widget enabled (a cw-payroll-factory) for token distribution to work correctly.
2. The contract uses cw_ownable for ownership management, allowing only the owner (DAO) to perform certain actions.
3. The vesting configuration can be updated, providing flexibility in token distribution strategies.