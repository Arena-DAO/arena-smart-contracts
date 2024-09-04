# Arena Payment Registry

## Overview

The Arena Payment Registry is a smart contract designed to integrate with the Arena Core. It allows users to define how rewards are further distributed within competitions. This registry is particularly useful for scenarios where teams or groups want to automatically distribute rewards among their members without requiring additional actions after a competition concludes.

## Key Features

- Integration with Arena Core for competition distributions
- User-defined reward distribution settings
- Immutable distribution records based on competition activation height
- Automatic payout to team members or predefined recipients

## Key Functions

### ExecuteMsg

1. `SetDistribution { distribution }`: Set a new distribution for the sender.
2. `RemoveDistribution {}`: Remove the existing distribution for the sender.

### QueryMsg

1. `GetDistribution { addr, height }`: Retrieve the distribution for a given address at a specific block height.

## Usage

1. **Setting a Distribution**: 
   Teams or individuals can set their desired distribution using the `SetDistribution` function.

2. **Removing a Distribution**: 
   Existing distributions can be removed using the `RemoveDistribution` function.

3. **Querying Distributions**: 
   The Arena Core or other authorized contracts can query the registry using `GetDistribution` to determine how to distribute rewards.

## Important Note

The distribution is determined based on the state of the registry at the competition's activation height (when it was fully funded). Once set for a particular competition, this distribution is immutable to ensure fairness and predictability.