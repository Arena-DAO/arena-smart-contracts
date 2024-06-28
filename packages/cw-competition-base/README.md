# cw-competition-base

`cw-competition-base` provides a flexible and extensible base for implementing different types of competitions such as wagers, leagues, and tournaments.

## Overview

This contract defines the core structure and functionality common to all competition types, allowing for easy extension and customization for specific competition modules.

## Features

- Generic base for competition instantiation and management
- Customizable competition parameters
- Support for competition jailing and activation
- Evidence submission functionality
- Competition processing and result management
- Extensible design for specific competition types

## Key Components

### InstantiateBase

Defines the basic structure for instantiating a competition module:

- `key`: Identifier for the competition type (e.g., "Wagers", "Tournaments", "Leagues")
- `description`: Description of the competition module
- `extension`: Custom instantiation parameters for specific competition types

### ExecuteBase

Provides a set of standard execute messages for competition management:

- `JailCompetition`: Jail a competition for review
- `ActivateCompetition`: Activate a competition
- `AddCompetitionHook` / `RemoveCompetitionHook`: Manage competition hooks
- `CreateCompetition`: Create a new competition
- `SubmitEvidence`: Submit evidence for a competition
- `ProcessCompetition`: Process the results of a competition
- `Extension`: Execute custom messages for specific competition types

### QueryBase

Offers standard query messages for retrieving competition information:

- `Config`: Get the configuration of the competition module
- `DAO`: Get the associated DAO address
- `CompetitionCount`: Get the total number of competitions
- `Competition`: Get details of a specific competition
- `Competitions`: List competitions with optional filtering
- `Evidence`: Retrieve evidence for a competition
- `Result`: Get the result of a competition
- `QueryExtension`: Custom queries for specific competition types