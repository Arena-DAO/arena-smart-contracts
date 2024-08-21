use arena_interface::core::{ExecuteExt, ExecuteMsg, QueryExtFns};
use cosmwasm_std::{to_json_binary, CosmosMsg, Decimal, Uint128, WasmMsg};
use cw_orch::{anyhow, prelude::*};
use dao_proposal_sudo::msg::ExecuteMsgFns as _;

use crate::Arena;

use super::{ADMIN, PREFIX};

#[test]
fn test_create_category() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let admin = mock.addr_make(ADMIN);
    let mut arena = Arena::deploy_on(mock.clone(), admin.clone())?;
    mock.next_block()?;

    // Create a new category
    arena.dao_dao.dao_proposal_sudo.set_sender(&admin);
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&arena_interface::core::ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateCategories {
                    to_add: Some(vec![arena_interface::core::NewCompetitionCategory {
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

    Ok(())
}

#[test]
fn test_disable_category() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let admin = mock.addr_make(ADMIN);
    let mut arena = Arena::deploy_on(mock.clone(), admin.clone())?;
    mock.next_block()?;

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

    Ok(())
}

#[test]
fn test_update_tax() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let admin = mock.addr_make(ADMIN);
    let mut arena = Arena::deploy_on(mock.clone(), admin.clone())?;
    mock.next_block()?;

    // Update tax
    arena.dao_dao.dao_proposal_sudo.set_sender(&admin);
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&arena_interface::core::ExecuteMsg::Extension {
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

    Ok(())
}

#[test]
fn test_competition_modules() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let admin = mock.addr_make(ADMIN);
    let mut arena = Arena::deploy_on(mock.clone(), admin.clone())?;
    mock.next_block()?;

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
            msg: to_json_binary(&arena_interface::core::ExecuteMsg::Extension {
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

    Ok(())
}
