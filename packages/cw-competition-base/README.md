# cw-competition-base

`cw-competition-base` provides a flexible and extensible base for implementing different types of competitions such as wagers, leagues, and tournaments.

## Key Components

### InstantiateBase

Defines the basic structure for instantiating a competition module:

- `key`: Identifier for the competition type (e.g., "Wagers", "Tournaments", "Leagues")
- `description`: Description of the competition module
- `extension`: Custom instantiation parameters for specific competition types

### ExecuteBase

Provides a set of standard execute messages for competition management:

- `JailCompetition`: Jail a competition for review
  - `competition_id`: Unique identifier of the competition
  - `title`: Title of the jailed competition
  - `description`: Description of the jailed competition
  - `distribution`: Optional distribution of the jailed competition's funds
  - `additional_layered_fees`: Optional additional fees for the jailed competition
- `ActivateCompetition`: Activate a competition
- `AddCompetitionHook` / `RemoveCompetitionHook`: Manage competition hooks
  - `competition_id`: Unique identifier of the competition
- `ExecuteCompetitionHook`: Execute a competition hook
  - `competition_id`: Unique identifier of the competition
  - `distribution`: Optional distribution of the competition's funds
- `CreateCompetition`: Create a new competition
  - `host`: Optional host of the competition (defaults to `info.sender`)
  - `category_id`: Optional category identifier for the competition
  - `escrow`: Optional escrow information for the competition
  - `name`: Name of the competition
  - `description`: Description of the competition
  - `expiration`: Expiration time of the competition
  - `rules`: Optional rules of the competition
  - `rulesets`: Optional rulesets for the competition
  - `banner`: Optional banner for the competition
  - `instantiate_extension`: Custom instantiation parameters for the competition
- `SubmitEvidence`: Submit evidence for a competition
  - `competition_id`: Unique identifier of the competition
  - `evidence`: Evidence to be submitted
- `ProcessCompetition`: Process the results of a competition
  - `competition_id`: Unique identifier of the competition
  - `distribution`: Optional distribution of the competition's funds
- `Extension`: Execute custom messages for specific competition types
- `MigrateEscrows`: Migrate escrows associated with competitions
  - `start_after`: Optional pagination start point
  - `limit`: Optional pagination limit
  - `filter`: Optional filter for competitions
  - `escrow_code_id`: Code ID of the escrow contract
  - `escrow_migrate_msg`: Migration message for the escrow contract

### QueryBase

Offers standard query messages for retrieving competition information:

- `Config`: Get the configuration of the competition module
- `DAO`: Get the associated DAO address
- `CompetitionCount`: Get the total number of competitions
- `Competition`: Get details of a specific competition
  - `competition_id`: Unique identifier of the competition
- `Competitions`: List competitions with optional filtering
  - `start_after`: Optional pagination start point
  - `limit`: Optional pagination limit
  - `filter`: Optional filter for competitions
- `Evidence`: Retrieve evidence for a competition
  - `competition_id`: Unique identifier of the competition
  - `start_after`: Optional pagination start point
  - `limit`: Optional pagination limit
- `Result`: Get the result of a competition
  - `competition_id`: Unique identifier of the competition
- `QueryExtension`: Custom queries for specific competition types
- `PaymentRegistry`: Get the payment registry address