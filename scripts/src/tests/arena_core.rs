use arena_interface::core::{
    ExecuteExt, ExecuteMsg, NewCompetitionCategory, NewRuleset, QueryExtFns,
};
use cosmwasm_std::{to_json_binary, CosmosMsg, Decimal, Uint128, WasmMsg};
use cw_orch::{anyhow, prelude::*};
use cw_utils::Duration;
use dao_proposal_sudo::msg::ExecuteMsgFns as _;

use crate::tests::helpers::setup_arena;

use super::PREFIX;

#[test]
fn test_create_category() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Create a new category
    arena.dao_dao.dao_proposal_sudo.set_sender(&admin);
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateCategories {
                    to_add: Some(vec![NewCompetitionCategory {
                        name: "New Category".to_string(),
                    }]),
                    to_edit: None,
                },
            })?,
            funds: vec![],
        })])?;

    // Query categories
    let categories = arena.arena_core.categories(None, None, None)?;
    assert_eq!(categories.len(), 3); // 2 initial categories + 1 new
    assert_eq!(categories[2].name, "New Category");
    assert_eq!(categories[2].id, Uint128::new(3));
    assert!(categories[2].is_enabled);

    // Try to create a category with an empty name (should fail)
    let result = arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateCategories {
                    to_add: Some(vec![NewCompetitionCategory {
                        name: "".to_string(),
                    }]),
                    to_edit: None,
                },
            })?,
            funds: vec![],
        })]);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_update_category() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Update a category name
    arena.dao_dao.dao_proposal_sudo.set_sender(&admin);
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateCategories {
                    to_add: None,
                    to_edit: Some(vec![arena_interface::core::EditCompetitionCategory::Edit {
                        category_id: Uint128::one(),
                        name: "Updated Category".to_string(),
                    }]),
                },
            })?,
            funds: vec![],
        })])?;

    // Query categories
    let categories = arena.arena_core.categories(None, None, None)?;
    assert_eq!(categories[0].name, "Updated Category");

    Ok(())
}

#[test]
fn test_disable_category() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Disable a category
    arena.dao_dao.dao_proposal_sudo.set_sender(&admin);
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateCategories {
                    to_add: None,
                    to_edit: Some(vec![
                        arena_interface::core::EditCompetitionCategory::Disable {
                            category_id: Uint128::one(),
                        },
                    ]),
                },
            })?,
            funds: vec![],
        })])?;

    // Query categories including disabled
    let categories = arena.arena_core.categories(Some(true), None, None)?;
    assert!(!categories[0].is_enabled);

    // Query only enabled categories
    let enabled_categories = arena.arena_core.categories(None, None, None)?;
    assert_eq!(enabled_categories.len(), 1);

    Ok(())
}

#[test]
fn test_update_tax() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Update tax
    arena.dao_dao.dao_proposal_sudo.set_sender(&admin);
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateTax {
                    tax: Decimal::percent(10),
                },
            })?,
            funds: vec![],
        })])?;
    mock.next_block()?;

    // Query tax
    let tax = arena.arena_core.tax(None)?;
    assert_eq!(tax, Decimal::percent(10));

    // Try to set an invalid tax (>100%)
    let result = arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateTax {
                    tax: Decimal::percent(101),
                },
            })?,
            funds: vec![],
        })]);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_competition_modules() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Query competition modules
    let modules = arena.arena_core.competition_modules(None, None, None)?;
    assert_eq!(modules.len(), 3); // Tournament, Wager, and League modules

    // Disable a module
    arena.dao_dao.dao_proposal_sudo.set_sender(&admin);
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateCompetitionModules {
                    to_add: None,
                    to_disable: Some(vec![modules[0].addr.to_string()]),
                },
            })?,
            funds: vec![],
        })])?;
    mock.next_block()?;

    // Query modules again
    let updated_modules = arena
        .arena_core
        .competition_modules(Some(true), None, None)?;
    assert!(!updated_modules[2].is_enabled); // The disabled modules are sent to the back

    // Try to disable a non-existent module (should fail)
    let result = arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateCompetitionModules {
                    to_add: None,
                    to_disable: Some(vec!["non_existent_module".to_string()]),
                },
            })?,
            funds: vec![],
        })]);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_update_rulesets() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Add a new ruleset
    arena.dao_dao.dao_proposal_sudo.set_sender(&admin);
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateRulesets {
                    to_add: Some(vec![NewRuleset {
                        category_id: Uint128::one(),
                        rules: vec!["New Rule 1".to_string(), "New Rule 2".to_string()],
                        description: "New Ruleset".to_string(),
                    }]),
                    to_disable: None,
                },
            })?,
            funds: vec![],
        })])?;
    mock.next_block()?;

    // Query rulesets
    let rulesets = arena
        .arena_core
        .rulesets(Uint128::one(), None, None, None)?;
    assert_eq!(rulesets.len(), 1);
    assert_eq!(rulesets[0].description, "New Ruleset");

    // Disable the ruleset
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateRulesets {
                    to_add: None,
                    to_disable: Some(vec![Uint128::one()]),
                },
            })?,
            funds: vec![],
        })])?;

    // Query rulesets again
    let updated_rulesets = arena
        .arena_core
        .rulesets(Uint128::one(), Some(true), None, None)?;
    assert!(!updated_rulesets[0].is_enabled);

    Ok(())
}

#[test]
fn test_update_rating_period() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Update rating period
    arena.dao_dao.dao_proposal_sudo.set_sender(&admin);
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateRatingPeriod {
                    period: Duration::Time(86400), // 1 day
                },
            })?,
            funds: vec![],
        })])?;

    // Query rating period
    let rating_period = arena.arena_core.rating_period()?;
    assert_eq!(rating_period, Some(Duration::Time(86400)));

    // Try to set an invalid rating period (0)
    let result = arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateRatingPeriod {
                    period: Duration::Time(0),
                },
            })?,
            funds: vec![],
        })]);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_update_enrollment_modules() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let new_module = mock.addr_make("new_enrollment_module");

    // Add a new enrollment module
    arena.dao_dao.dao_proposal_sudo.set_sender(&admin);
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateEnrollmentModules {
                    to_add: Some(vec![new_module.to_string()]),
                    to_remove: None,
                },
            })?,
            funds: vec![],
        })])?;

    // Ensure the module is now a valid enrollment module
    assert!(arena
        .arena_core
        .is_valid_enrollment_module(new_module.to_string())?);

    // Remove the enrollment module
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateEnrollmentModules {
                    to_add: None,
                    to_remove: Some(vec![new_module.to_string()]),
                },
            })?,
            funds: vec![],
        })])?;

    // Query enrollment modules again
    assert!(!arena
        .arena_core
        .is_valid_enrollment_module(new_module.to_string())?);

    // Try to remove a non-existent module (should fail)
    let result = arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateEnrollmentModules {
                    to_add: None,
                    to_remove: Some(vec!["non_existent_module".to_string()]),
                },
            })?,
            funds: vec![],
        })]);
    assert!(result.is_err());

    Ok(())
}
