# Arena Core

Arena Core is the core component of the Arena DAO ecosystem, managing competition modules, rulesets, categories, and ratings. It serves as the central hub for coordinating various aspects of decentralized competitions.

## Contract Messages

### InstantiateMsg

Initializes the Arena Core contract with the following parameters:

- Competition module instantiation information
- Initial rulesets
- Initial categories
- Tax rate and configuration
- Rating period duration

### ExecuteMsg

The contract supports the following execute messages:

- `UpdateCompetitionModules`: Add or disable competition modules
- `UpdateTax`: Modify the tax rate
- `UpdateRulesets`: Add or disable rulesets
- `UpdateCategories`: Add, edit, or disable competition categories
- `AdjustRatings`: Update ratings for participants in a specific category
- `UpdateRatingPeriod`: Modify the rating period duration
- `UpdateEnrollmentModules`: Add or remove enrollment modules

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