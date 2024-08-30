# Arena Core

Arena Core is the core component of the Arena DAO ecosystem, managing competition modules, rulesets, categories, and ratings. It serves as the central hub for coordinating various aspects of the decentralized competition platform.

This contract is a modified version of the dao-prepropose-base and can handle the 'jailing' of unresolved competitions. The proposal deposit for this functionality is configurable to avoid spam or dishonest proposals.

## Contract Messages

### InstantiateMsg

The contract is instantiated with the following parameters:

- `competition_modules_instantiate_info`: Optional vector of `ModuleInstantiateInfo` to initialize competition modules
- `rulesets`: Optional vector of `NewRuleset` to set up initial rulesets
- `categories`: Optional vector of `NewCompetitionCategory` to establish initial competition categories
- `tax`: Decimal value representing the tax rate
- `tax_configuration`: Configuration for tax application
- `rating_period`: Duration for the rating period
- `payment_registry`: Optional string to set the payment registry module

### ExecuteMsg

The contract supports the following execute messages:

- `UpdateCompetitionModules`: Add or disable competition modules
- `UpdateTax`: Modify the tax rate
- `UpdateRulesets`: Add or disable rulesets
- `UpdateCategories`: Add, edit, or disable competition categories
- `AdjustRatings`: Update ratings for participants in a specific category
- `UpdateRatingPeriod`: Modify the rating period duration
- `UpdateEnrollmentModules`: Add or remove enrollment modules
- `SetPaymentRegistry`: Sets the payment registry module

### QueryMsg

The contract supports various query messages:

- `CompetitionModules`: List competition modules
- `Ruleset`: Get details of a specific ruleset
- `Rulesets`: List rulesets for a category
- `Tax`: Get the current tax rate
- `CompetitionModule`: Get details of a specific competition module
- `Category`: Get details of a specific category
- `Categories`: List categories
- `IsValidCategoryAndRulesets`: Validate category and ruleset combinations
- `IsValidEnrollmentModule`: Check if an enrollment module is valid
- `DumpState`: Get the current state of the contract
- `TaxConfig`: Get tax configuration for a specific height
- `Rating`: Get rating for a participant in a category
- `RatingLeaderboard`: Get the rating leaderboard for a category
- `PaymentRegistry`: Get the payment registry module