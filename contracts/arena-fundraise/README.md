# Arena-Fundraise

Arena-Fundraise is a smart contract for raising funds within the initial parameters. This contract will only support native tokens.

The fundraiser will define the `fundraise_token` and `amount` to be paid on instantiation along with a `start` and `expiration`. 
Users will be able to deposit the `deposit_token` for a share of the allocated `fundraise_token`.
Additionally, if a `soft_cap` is defined but not reached upon expiration, then the fundraise will be considered failed with all deposited tokens available for withdrawal.
If a `hard_cap` is defined, users will not be able to deposit more than its amount.