# Arena-Escrow Smart Contract

## Overview

The Arena-Escrow smart contract is a key component of the Arena DAO ecosystem, managing the lifecycle of competitions and their associated funds. This contract handles dues collection, fund distribution, and competition state management.

## Competition and Escrow Lifecycle

The following flowchart illustrates the lifecycle of a competition and its associated escrow:

```mermaid
flowchart TB
    Start([Start]) --> HasDues{Competition<br>Has Dues?}
    HasDues -->|Yes| ShouldActivateDues{Should Activate<br>On Funded?}
    HasDues -->|No| ShouldActivateNoDues{Should Activate<br>On Funded?}
    
    ShouldActivateDues -->|Yes| PendingDues[Pending:<br>Awaiting Dues]
    ShouldActivateDues -->|No| PendingManual[Pending:<br>Manual Activation]
    
    ShouldActivateNoDues -->|Yes| ActiveCompetition[Active Competition]
    ShouldActivateNoDues -->|No| PendingManual
    
    PendingDues -->|All Dues Paid| ActiveCompetition
    PendingManual -->|Host Manually<br>Activates| ActiveCompetition
    
    ActiveCompetition -->|Expiration Reached| ConsensusCheck{Consensus<br>Reached?}
    ConsensusCheck -->|Yes| InactiveCompetition[Inactive Competition]
    ConsensusCheck -->|No| JailedCompetition[Jailed Competition]
    
    JailedCompetition -->|Arena DAO<br>Resolves| InactiveCompetition
    
    InactiveCompetition --> End([End])
    
    UnlockedEscrow[Escrow:<br>Unlocked] -->|Competition<br>Activated| LockedEscrow[Escrow:<br>Locked]
    LockedEscrow -->|Competition<br>Resolved| UnlockedEscrow
    
    PendingDues -.-> UnlockedEscrow
    PendingManual -.-> UnlockedEscrow
    ActiveCompetition -.-> LockedEscrow
    InactiveCompetition -.-> UnlockedEscrow
    
    classDef default fill:#4EA5D9,stroke:#2E4057,stroke-width:2px,color:#2E4057;
    classDef decision fill:#F99B45,stroke:#2E4057,stroke-width:2px,color:#2E4057;
    classDef escrow fill:#73D2DE,stroke:#2E4057,stroke-width:2px,color:#2E4057;
    classDef startend fill:#2ECC71,stroke:#2E4057,stroke-width:2px,color:#2E4057;
    
    class HasDues,ShouldActivateDues,ShouldActivateNoDues,ConsensusCheck decision;
    class UnlockedEscrow,LockedEscrow escrow;
    class Start,End startend;
```

## Key Features

- Dues collection and management
- Automatic or manual competition activation
- Fund locking and unlocking
- Distribution of funds based on competition results
- Support for native, CW20, and CW721 tokens
- Layered fee system

## Contract Messages

### InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub dues: Vec<MemberBalanceUnchecked>,
    pub should_activate_on_funded: Option<bool>,
}
```

### ExecuteMsg

The contract supports the following execute messages:

- `Withdraw`: Withdraw funds from the contract
- `SetDistribution`: Set the distribution of funds
- `Activate`: Activate the competition
- `ReceiveNative`: Receive native tokens
- `Receive`: Receive CW20 tokens
- `ReceiveNft`: Receive CW721 tokens
- `Distribute`: Distribute funds according to the specified distribution and fees
- `Lock`: Lock or unlock the contract

### QueryMsg

The contract supports various query messages to check balances, dues, funding status, and other state information.
