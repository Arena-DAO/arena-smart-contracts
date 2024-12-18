use arena_competition_enrollment::msg::{
    CompetitionInfoMsg, ExecuteMsg, ExecuteMsgFns as _, MigrateMsg, QueryMsgFns as _,
};
use arena_competition_enrollment::state::CompetitionType;
use arena_interface::competition::msg::QueryBaseFns as _;
use arena_interface::escrow::ExecuteMsgFns as _;
use arena_interface::group::{self, QueryMsgFns as _};
use arena_tournament_module::state::EliminationType;
use cosmwasm_std::{coins, to_json_binary, CosmosMsg, Decimal, Uint128, Uint64, WasmMsg};
use cw_orch::{anyhow, prelude::*};
use cw_orch_clone_testing::CloneTesting;
use cw_utils::Expiration;
use dao_interface::state::ModuleInstantiateInfo;
use dao_proposal_sudo::msg::ExecuteMsgFns as _;
use networks::PION_1;

use crate::arena::Arena;
use crate::tests::helpers::setup_arena;

use super::{DENOM, PREFIX};

#[test]
fn test_competition_enrollment() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Set teams
    let mut teams = vec![];
    for i in 0..10 {
        teams.push(mock.addr_make_with_balance(format!("team {}", i), coins(100_000u128, DENOM))?);
    }

    // Register the enrollment module
    arena.dao_dao.dao_proposal_sudo.set_sender(&admin);
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&arena_interface::core::ExecuteMsg::Extension {
                msg: arena_interface::core::ExecuteExt::UpdateEnrollmentModules {
                    to_add: Some(vec![arena.arena_competition_enrollment.addr_str()?]),
                    to_remove: None,
                },
            })?,
            funds: vec![],
        })])?;

    // Create an enrollment
    arena.arena_competition_enrollment.set_sender(&admin);
    let create_enrollment_msg = ExecuteMsg::CreateEnrollment {
        min_members: Some(Uint64::new(4)),
        max_members: Uint64::new(10),
        entry_fee: Some(coins(1000, DENOM)[0].clone()),
        expiration: Expiration::AtHeight(1000000),
        category_id: Some(Uint128::new(1)),
        competition_info: CompetitionInfoMsg {
            name: "Test Competition".to_string(),
            description: "A test competition".to_string(),
            expiration: Expiration::AtHeight(2000000),
            rules: Some(vec!["Rule 1".to_string(), "Rule 2".to_string()]),
            rulesets: None,
            banner: None,
            additional_layered_fees: None,
        },
        competition_type: CompetitionType::Tournament {
            elimination_type: EliminationType::SingleElimination {
                play_third_place_match: false,
            },
            distribution: vec![Decimal::percent(60), Decimal::percent(40)],
        },
        group_contract_info: ModuleInstantiateInfo {
            code_id: arena.arena_group.code_id()?,
            msg: to_json_binary(&group::InstantiateMsg { members: None })?,
            admin: None,
            funds: vec![],
            label: "Arena Group".to_string(),
        },
    };

    let res = arena
        .arena_competition_enrollment
        .execute(&create_enrollment_msg, None)?;
    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "create_enrollment")));

    // Test the query
    let enrollments = arena
        .arena_competition_enrollment
        .enrollments(None, None, None)?;
    assert!(enrollments.len() == 1);

    // Set group contract
    arena
        .arena_group
        .set_address(&enrollments[0].group_contract);

    // Enroll a member
    arena.arena_competition_enrollment.set_sender(&teams[0]);

    let res = arena
        .arena_competition_enrollment
        .enroll(Uint128::one(), &coins(1000, DENOM))?;
    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "enroll")));

    // Query members
    let members = arena.arena_group.members(None, None)?;
    assert!(members.len() == 1);

    // Try to enroll the same member again (should fail)
    let result = arena
        .arena_competition_enrollment
        .enroll(Uint128::one(), &coins(1000, DENOM));
    assert!(result.is_err());

    // Withdraw a member before expiration
    let res = arena
        .arena_competition_enrollment
        .withdraw(Uint128::one())?;
    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "withdraw")));

    // Enroll again
    arena
        .arena_competition_enrollment
        .enroll(Uint128::one(), &coins(1000, DENOM))?;

    // Trigger expiration
    arena.arena_competition_enrollment.set_sender(&admin);
    mock.wait_blocks(1000000)?; // Move to expiration block

    let res = arena
        .arena_competition_enrollment
        .trigger_expiration(arena.arena_escrow.code_id()?, Uint128::one())?;
    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "trigger_expiration")));

    // The enrollment didn't succeed, so we should be able to withdraw
    arena.arena_competition_enrollment.set_sender(&teams[0]);
    arena
        .arena_competition_enrollment
        .withdraw(Uint128::one())?;

    Ok(())
}

#[test]
fn test_invalid_enrollment() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Set teams
    let mut teams = vec![];
    for i in 0..10 {
        teams.push(mock.addr_make_with_balance(format!("team {}", i), coins(100_000u128, DENOM))?);
    }

    arena.arena_competition_enrollment.set_sender(&admin);

    // Try to create an invalid enrollment (min_members > max_members)
    let invalid_enrollment_msg = ExecuteMsg::CreateEnrollment {
        min_members: Some(Uint64::new(11)),
        max_members: Uint64::new(10),
        entry_fee: Some(coins(1000, DENOM)[0].clone()),
        expiration: Expiration::AtHeight(1000000),
        category_id: Some(Uint128::new(1)),
        competition_info: CompetitionInfoMsg {
            name: "Invalid Competition".to_string(),
            description: "An invalid competition".to_string(),
            expiration: Expiration::AtHeight(2000000),
            rules: None,
            rulesets: None,
            banner: None,
            additional_layered_fees: None,
        },
        competition_type: CompetitionType::Tournament {
            elimination_type: EliminationType::SingleElimination {
                play_third_place_match: false,
            },
            distribution: vec![Decimal::percent(60), Decimal::percent(40)],
        },
        group_contract_info: ModuleInstantiateInfo {
            code_id: arena.arena_group.code_id()?,
            msg: to_json_binary(&group::InstantiateMsg { members: None })?,
            admin: None,
            funds: vec![],
            label: "Arena Group".to_string(),
        },
    };

    let result = arena
        .arena_competition_enrollment
        .execute(&invalid_enrollment_msg, None);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_enrollment_capacity() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Set teams
    let mut teams = vec![];
    for i in 0..5 {
        teams.push(mock.addr_make_with_balance(format!("team {}", i), coins(100_000u128, DENOM))?);
    }

    // Register the enrollment module
    arena.dao_dao.dao_proposal_sudo.set_sender(&admin);
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&arena_interface::core::ExecuteMsg::Extension {
                msg: arena_interface::core::ExecuteExt::UpdateEnrollmentModules {
                    to_add: Some(vec![arena.arena_competition_enrollment.addr_str()?]),
                    to_remove: None,
                },
            })?,
            funds: vec![],
        })])?;

    arena.arena_competition_enrollment.set_sender(&admin);

    // Create an enrollment with max_members = 4
    let create_enrollment_msg = ExecuteMsg::CreateEnrollment {
        min_members: Some(Uint64::new(4)),
        max_members: Uint64::new(4),
        entry_fee: Some(coins(1000, DENOM)[0].clone()),
        expiration: Expiration::AtHeight(1000000),
        category_id: Some(Uint128::new(1)),
        competition_info: CompetitionInfoMsg {
            name: "Capacity Test".to_string(),
            description: "Testing enrollment capacity".to_string(),
            expiration: Expiration::AtHeight(2000000),
            rules: None,
            rulesets: None,
            banner: None,
            additional_layered_fees: None,
        },
        competition_type: CompetitionType::Tournament {
            elimination_type: EliminationType::SingleElimination {
                play_third_place_match: false,
            },
            distribution: vec![Decimal::percent(60), Decimal::percent(40)],
        },
        group_contract_info: ModuleInstantiateInfo {
            code_id: arena.arena_group.code_id()?,
            msg: to_json_binary(&group::InstantiateMsg { members: None })?,
            admin: None,
            funds: vec![],
            label: "Arena Group".to_string(),
        },
    };

    arena
        .arena_competition_enrollment
        .execute(&create_enrollment_msg, None)?;

    // Enroll four members
    for team in teams.iter().take(4) {
        arena.arena_competition_enrollment.set_sender(team);
        arena
            .arena_competition_enrollment
            .enroll(Uint128::one(), &coins(1000, DENOM))?;
    }

    // Try to enroll a fifth member (should fail)
    arena.arena_competition_enrollment.set_sender(&teams[4]);

    let result = arena
        .arena_competition_enrollment
        .enroll(Uint128::one(), &coins(1000, DENOM));
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_successful_tournament_creation() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Set teams
    let mut teams = vec![];
    for i in 0..8 {
        teams.push(mock.addr_make_with_balance(format!("team {}", i), coins(100_000u128, DENOM))?);
    }

    // Register the enrollment module
    arena.dao_dao.dao_proposal_sudo.set_sender(&admin);
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&arena_interface::core::ExecuteMsg::Extension {
                msg: arena_interface::core::ExecuteExt::UpdateEnrollmentModules {
                    to_add: Some(vec![arena.arena_competition_enrollment.addr_str()?]),
                    to_remove: None,
                },
            })?,
            funds: vec![],
        })])?;

    // Create a tournament enrollment
    arena.arena_competition_enrollment.set_sender(&admin);
    let create_enrollment_msg = ExecuteMsg::CreateEnrollment {
        min_members: Some(Uint64::new(4)),
        max_members: Uint64::new(8),
        entry_fee: Some(coins(1000, DENOM)[0].clone()),
        expiration: Expiration::AtHeight(1000000),
        category_id: Some(Uint128::new(1)),
        competition_info: CompetitionInfoMsg {
            name: "Test Tournament".to_string(),
            description: "A test tournament".to_string(),
            expiration: Expiration::AtHeight(2000000),
            rules: Some(vec!["Tournament Rule".to_string()]),
            rulesets: None,
            banner: None,
            additional_layered_fees: None,
        },
        competition_type: CompetitionType::Tournament {
            elimination_type: EliminationType::SingleElimination {
                play_third_place_match: false,
            },
            distribution: vec![Decimal::percent(60), Decimal::percent(40)],
        },
        group_contract_info: ModuleInstantiateInfo {
            code_id: arena.arena_group.code_id()?,
            msg: to_json_binary(&group::InstantiateMsg { members: None })?,
            admin: None,
            funds: vec![],
            label: "Arena Group".to_string(),
        },
    };

    arena
        .arena_competition_enrollment
        .execute(&create_enrollment_msg, None)?;

    // Enroll 8 members
    for team in &teams {
        arena.arena_competition_enrollment.set_sender(team);
        arena
            .arena_competition_enrollment
            .enroll(Uint128::one(), &coins(1000, DENOM))?;
    }

    // Trigger expiration
    arena.arena_competition_enrollment.set_sender(&admin);
    mock.wait_blocks(1000000)?; // Move to expiration block

    let res = arena
        .arena_competition_enrollment
        .trigger_expiration(arena.arena_escrow.code_id()?, Uint128::one())?;
    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "trigger_expiration")));
    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "result" && attr.value == "competition_created")));

    Ok(())
}

#[test]
fn test_successful_wager_creation() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Set teams
    let team1 = mock.addr_make_with_balance("team 1", coins(100_000u128, DENOM))?;
    let team2 = mock.addr_make_with_balance("team 2", coins(100_000u128, DENOM))?;

    // Register the enrollment module
    arena.dao_dao.dao_proposal_sudo.set_sender(&admin);
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&arena_interface::core::ExecuteMsg::Extension {
                msg: arena_interface::core::ExecuteExt::UpdateEnrollmentModules {
                    to_add: Some(vec![arena.arena_competition_enrollment.addr_str()?]),
                    to_remove: None,
                },
            })?,
            funds: vec![],
        })])?;

    // Create a wager enrollment
    arena.arena_competition_enrollment.set_sender(&admin);
    let create_enrollment_msg = ExecuteMsg::CreateEnrollment {
        min_members: Some(Uint64::new(2)),
        max_members: Uint64::new(2),
        entry_fee: Some(coins(1000, DENOM)[0].clone()),
        expiration: Expiration::AtHeight(1000000),
        category_id: Some(Uint128::new(1)),
        competition_info: CompetitionInfoMsg {
            name: "Test Wager".to_string(),
            description: "A test wager".to_string(),
            expiration: Expiration::AtHeight(2000000),
            rules: Some(vec!["Wager Rule".to_string()]),
            rulesets: None,
            banner: None,
            additional_layered_fees: None,
        },
        competition_type: CompetitionType::Wager {},
        group_contract_info: ModuleInstantiateInfo {
            code_id: arena.arena_group.code_id()?,
            msg: to_json_binary(&group::InstantiateMsg { members: None })?,
            admin: None,
            funds: vec![],
            label: "Arena Group".to_string(),
        },
    };

    arena
        .arena_competition_enrollment
        .execute(&create_enrollment_msg, None)?;

    // Enroll 2 members
    arena.arena_competition_enrollment.set_sender(&team1);
    arena
        .arena_competition_enrollment
        .enroll(Uint128::one(), &coins(1000, DENOM))?;
    arena.arena_competition_enrollment.set_sender(&team2);
    arena
        .arena_competition_enrollment
        .enroll(Uint128::one(), &coins(1000, DENOM))?;

    // Trigger expiration
    arena.arena_competition_enrollment.set_sender(&admin);
    mock.wait_blocks(1000000)?; // Move to expiration block

    let res = arena
        .arena_competition_enrollment
        .trigger_expiration(arena.arena_escrow.code_id()?, Uint128::one())?;
    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "trigger_expiration")));
    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "result" && attr.value == "competition_created")));

    Ok(())
}

#[test]
fn test_successful_league_creation() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Set teams
    let mut teams = vec![];
    for i in 0..6 {
        teams.push(mock.addr_make_with_balance(format!("team {}", i), coins(100_000u128, DENOM))?);
    }

    // Register the enrollment module
    arena.dao_dao.dao_proposal_sudo.set_sender(&admin);
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&arena_interface::core::ExecuteMsg::Extension {
                msg: arena_interface::core::ExecuteExt::UpdateEnrollmentModules {
                    to_add: Some(vec![arena.arena_competition_enrollment.addr_str()?]),
                    to_remove: None,
                },
            })?,
            funds: vec![],
        })])?;

    // Create a league enrollment
    arena.arena_competition_enrollment.set_sender(&admin);
    let create_enrollment_msg = ExecuteMsg::CreateEnrollment {
        min_members: Some(Uint64::new(4)),
        max_members: Uint64::new(6),
        entry_fee: Some(coins(1000, DENOM)[0].clone()),
        expiration: Expiration::AtHeight(1000000),
        category_id: Some(Uint128::new(1)),
        competition_info: CompetitionInfoMsg {
            name: "Test League".to_string(),
            description: "A test league".to_string(),
            expiration: Expiration::AtHeight(2000000),
            rules: Some(vec!["League Rule".to_string()]),
            rulesets: None,
            banner: None,
            additional_layered_fees: None,
        },
        competition_type: CompetitionType::League {
            match_win_points: Uint64::new(3),
            match_draw_points: Uint64::new(1),
            match_lose_points: Uint64::new(0),
            distribution: vec![
                Decimal::percent(50),
                Decimal::percent(30),
                Decimal::percent(20),
            ],
        },
        group_contract_info: ModuleInstantiateInfo {
            code_id: arena.arena_group.code_id()?,
            msg: to_json_binary(&group::InstantiateMsg { members: None })?,
            admin: None,
            funds: vec![],
            label: "Arena Group".to_string(),
        },
    };

    arena
        .arena_competition_enrollment
        .execute(&create_enrollment_msg, None)?;

    // Enroll 6 members
    for team in &teams {
        arena.arena_competition_enrollment.set_sender(team);
        arena
            .arena_competition_enrollment
            .enroll(Uint128::one(), &coins(1000, DENOM))?;
    }

    // Trigger expiration
    arena.arena_competition_enrollment.set_sender(&admin);
    mock.wait_blocks(1000000)?; // Move to expiration block

    let res = arena
        .arena_competition_enrollment
        .trigger_expiration(arena.arena_escrow.code_id()?, Uint128::one())?;
    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "trigger_expiration")));
    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "result" && attr.value == "competition_created")));

    Ok(())
}

#[test]
fn test_trigger_expiration_without_escrow() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Register the enrollment module
    arena.dao_dao.dao_proposal_sudo.set_sender(&admin);
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&arena_interface::core::ExecuteMsg::Extension {
                msg: arena_interface::core::ExecuteExt::UpdateEnrollmentModules {
                    to_add: Some(vec![arena.arena_competition_enrollment.addr_str()?]),
                    to_remove: None,
                },
            })?,
            funds: vec![],
        })])?;

    // Create an enrollment
    arena.arena_competition_enrollment.set_sender(&admin);
    let create_enrollment_msg = ExecuteMsg::CreateEnrollment {
        min_members: None,
        max_members: Uint64::new(10),
        entry_fee: None,
        expiration: Expiration::AtHeight(1000000),
        category_id: Some(Uint128::new(1)),
        competition_info: CompetitionInfoMsg {
            name: "Test Competition".to_string(),
            description: "A test competition".to_string(),
            expiration: Expiration::AtHeight(2000000),
            rules: Some(vec!["Rule 1".to_string()]),
            rulesets: None,
            banner: None,
            additional_layered_fees: None,
        },
        competition_type: CompetitionType::Wager {},
        group_contract_info: ModuleInstantiateInfo {
            code_id: arena.arena_group.code_id()?,
            msg: to_json_binary(&group::InstantiateMsg { members: None })?,
            admin: None,
            funds: vec![],
            label: "Arena Group".to_string(),
        },
    };

    arena
        .arena_competition_enrollment
        .execute(&create_enrollment_msg, None)?;

    // Enroll 3 members
    let mut teams = vec![];
    for i in 0..3 {
        let team = mock.addr_make(format!("team {}", i));
        teams.push(team.clone());
        arena.arena_competition_enrollment.set_sender(&team);
        arena
            .arena_competition_enrollment
            .enroll(Uint128::one(), &[])?;
    }

    // Trigger expiration
    arena.arena_competition_enrollment.set_sender(&admin);
    mock.wait_blocks(1000000)?; // Move to expiration block

    let res = arena
        .arena_competition_enrollment
        .trigger_expiration(arena.arena_escrow.code_id()?, Uint128::one())?;

    // Check that the competition was created
    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "trigger_expiration")));
    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "result" && attr.value == "competition_created")));

    Ok(())
}

#[test]
fn test_trigger_expiration_before_min_members() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Register the enrollment module
    arena.dao_dao.dao_proposal_sudo.set_sender(&admin);
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&arena_interface::core::ExecuteMsg::Extension {
                msg: arena_interface::core::ExecuteExt::UpdateEnrollmentModules {
                    to_add: Some(vec![arena.arena_competition_enrollment.addr_str()?]),
                    to_remove: None,
                },
            })?,
            funds: vec![],
        })])?;

    // Create an enrollment with min_members = 4
    arena.arena_competition_enrollment.set_sender(&admin);
    let create_enrollment_msg = ExecuteMsg::CreateEnrollment {
        min_members: Some(Uint64::new(4)),
        max_members: Uint64::new(10),
        entry_fee: Some(coins(1000, DENOM)[0].clone()),
        expiration: Expiration::AtHeight(1000000),
        category_id: Some(Uint128::new(1)),
        competition_info: CompetitionInfoMsg {
            name: "Test Competition".to_string(),
            description: "A test competition".to_string(),
            expiration: Expiration::AtHeight(2000000),
            rules: Some(vec!["Rule 1".to_string()]),
            rulesets: None,
            banner: None,
            additional_layered_fees: None,
        },
        competition_type: CompetitionType::Tournament {
            elimination_type: EliminationType::SingleElimination {
                play_third_place_match: false,
            },
            distribution: vec![Decimal::percent(60), Decimal::percent(40)],
        },
        group_contract_info: ModuleInstantiateInfo {
            code_id: arena.arena_group.code_id()?,
            msg: to_json_binary(&group::InstantiateMsg { members: None })?,
            admin: None,
            funds: vec![],
            label: "Arena Group".to_string(),
        },
    };

    arena
        .arena_competition_enrollment
        .execute(&create_enrollment_msg, None)?;

    // Enroll only 3 members
    let mut teams = vec![];
    for i in 0..3 {
        let team = mock.addr_make_with_balance(format!("team {}", i), coins(100_000u128, DENOM))?;
        teams.push(team.clone());
        arena.arena_competition_enrollment.set_sender(&team);
        arena
            .arena_competition_enrollment
            .enroll(Uint128::one(), &coins(1000, DENOM))?;
    }

    // Trigger expiration
    arena.arena_competition_enrollment.set_sender(&admin);
    mock.wait_blocks(1000000)?; // Move to expiration block

    let res = arena
        .arena_competition_enrollment
        .trigger_expiration(arena.arena_escrow.code_id()?, Uint128::one())?;

    // Check that the competition was not created
    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "result" && attr.value == "expired_insufficient_members")));

    // Attempt to withdraw for each enrolled team
    for team in teams {
        arena.arena_competition_enrollment.set_sender(&team);
        let withdraw_res = arena
            .arena_competition_enrollment
            .withdraw(Uint128::one())?;

        // Check that the withdrawal was successful
        assert!(withdraw_res.events.iter().any(|e| e.ty == "wasm"
            && e.attributes
                .iter()
                .any(|attr| attr.key == "action" && attr.value == "withdraw")));
    }

    Ok(())
}

#[test]
fn test_unregistered_competition_enrollment() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Create an enrollment with min_members = 4
    arena.arena_competition_enrollment.set_sender(&admin);
    let create_enrollment_msg = ExecuteMsg::CreateEnrollment {
        min_members: Some(Uint64::new(4)),
        max_members: Uint64::new(10),
        entry_fee: Some(coins(1000, DENOM)[0].clone()),
        expiration: Expiration::AtHeight(1000000),
        category_id: Some(Uint128::new(1)),
        competition_info: CompetitionInfoMsg {
            name: "Test Competition".to_string(),
            description: "A test competition".to_string(),
            expiration: Expiration::AtHeight(2000000),
            rules: Some(vec!["Rule 1".to_string()]),
            rulesets: None,
            banner: None,
            additional_layered_fees: None,
        },
        competition_type: CompetitionType::Tournament {
            elimination_type: EliminationType::SingleElimination {
                play_third_place_match: false,
            },
            distribution: vec![Decimal::percent(60), Decimal::percent(40)],
        },
        group_contract_info: ModuleInstantiateInfo {
            code_id: arena.arena_group.code_id()?,
            msg: to_json_binary(&group::InstantiateMsg { members: None })?,
            admin: None,
            funds: vec![],
            label: "Arena Group".to_string(),
        },
    };

    arena
        .arena_competition_enrollment
        .execute(&create_enrollment_msg, None)?;

    // Enroll only minimum number of members
    let mut teams = vec![];
    for i in 0..4 {
        let team = mock.addr_make_with_balance(format!("team {}", i), coins(100_000u128, DENOM))?;
        teams.push(team.clone());
        arena.arena_competition_enrollment.set_sender(&team);
        arena
            .arena_competition_enrollment
            .enroll(Uint128::one(), &coins(1000, DENOM))?;
    }

    // Trigger expiration
    arena.arena_competition_enrollment.set_sender(&admin);
    mock.wait_blocks(1000000)?; // Move to expiration block

    let res = arena
        .arena_competition_enrollment
        .trigger_expiration(arena.arena_escrow.code_id()?, Uint128::one())?;

    // Check that the competition was not created due to reply on error
    assert!(res
        .events
        .iter()
        .any(|e| e.ty == "wasm" && e.attributes.iter().any(|attr| attr.key == "error")));

    // Attempt to withdraw for each enrolled team
    for team in teams {
        arena.arena_competition_enrollment.set_sender(&team);
        let withdraw_res = arena
            .arena_competition_enrollment
            .withdraw(Uint128::one())?;

        // Check that the withdrawal was successful
        assert!(withdraw_res.events.iter().any(|e| e.ty == "wasm"
            && e.attributes
                .iter()
                .any(|attr| attr.key == "action" && attr.value == "withdraw")));
    }

    Ok(())
}

#[test]
fn test_huge_tournament() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Set teams
    let mut teams = vec![];
    for i in 0..10000 {
        teams.push(mock.addr_make_with_balance(format!("team {}", i), coins(100_000u128, DENOM))?);
    }

    // Register the enrollment module
    arena.dao_dao.dao_proposal_sudo.set_sender(&admin);
    arena
        .dao_dao
        .dao_proposal_sudo
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_core.addr_str()?,
            msg: to_json_binary(&arena_interface::core::ExecuteMsg::Extension {
                msg: arena_interface::core::ExecuteExt::UpdateEnrollmentModules {
                    to_add: Some(vec![arena.arena_competition_enrollment.addr_str()?]),
                    to_remove: None,
                },
            })?,
            funds: vec![],
        })])?;

    // Create a tournament enrollment
    arena.arena_competition_enrollment.set_sender(&admin);
    let create_enrollment_msg = ExecuteMsg::CreateEnrollment {
        min_members: None,
        max_members: Uint64::new(10000),
        entry_fee: Some(coins(1000, DENOM)[0].clone()),
        expiration: Expiration::AtHeight(1000000),
        category_id: Some(Uint128::new(1)),
        competition_info: CompetitionInfoMsg {
            name: "Test Tournament".to_string(),
            description: "A test tournament".to_string(),
            expiration: Expiration::AtHeight(2000000),
            rules: Some(vec!["Tournament Rule".to_string()]),
            rulesets: None,
            banner: None,
            additional_layered_fees: None,
        },
        competition_type: CompetitionType::Tournament {
            elimination_type: EliminationType::SingleElimination {
                play_third_place_match: false,
            },
            distribution: vec![Decimal::percent(60), Decimal::percent(40)],
        },
        group_contract_info: ModuleInstantiateInfo {
            code_id: arena.arena_group.code_id()?,
            msg: to_json_binary(&group::InstantiateMsg { members: None })?,
            admin: None,
            funds: vec![],
            label: "Arena Group".to_string(),
        },
    };

    arena
        .arena_competition_enrollment
        .execute(&create_enrollment_msg, None)?;

    // Enroll all 10000 members
    for team in &teams {
        arena.arena_competition_enrollment.set_sender(team);
        arena
            .arena_competition_enrollment
            .enroll(Uint128::one(), &coins(1000, DENOM))?;
    }

    // Trigger expiration
    arena.arena_competition_enrollment.set_sender(&admin);
    mock.wait_blocks(1000000)?; // Move to expiration block

    let res = arena
        .arena_competition_enrollment
        .trigger_expiration(arena.arena_escrow.code_id()?, Uint128::one())?;
    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "trigger_expiration")));
    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "result" && attr.value == "competition_created")));

    // Test tipping
    let competition = arena.arena_tournament_module.competition(Uint128::one())?;
    arena.arena_escrow.set_sender(&teams[0]);
    arena
        .arena_escrow
        .set_address(competition.escrow.as_ref().unwrap());
    arena
        .arena_escrow
        .receive_native(coins(5000u128, DENOM).as_slice())?;

    Ok(())
}

#[test]
#[ignore = "RPC blocks"]
fn test_migration_v2_v2_1() -> anyhow::Result<()> {
    let app = CloneTesting::new(PION_1)?;
    let mut arena = Arena::new(app.clone());
    const ARENA_DAO: &str = "neutron1ehkcl0n6s2jtdw75xsvfxm304mz4hs5z7jt6wn5mk0celpj0epqql4ulxk";
    let arena_dao_addr = Addr::unchecked(ARENA_DAO);

    arena.arena_group.upload()?;
    arena.arena_competition_enrollment.upload()?;

    arena
        .arena_competition_enrollment
        .set_address(&Addr::unchecked(
            "neutron16gtf438zpdu09zft6wdttcg5x648xwv88ljfw3gxgr4rjmfxlrdq7n4sxy",
        ));
    arena
        .arena_competition_enrollment
        .set_sender(&arena_dao_addr);

    arena
        .arena_competition_enrollment
        .migrate(&MigrateMsg::FromCompatible {}, 8475)?;

    let enrollments = arena
        .arena_competition_enrollment
        .enrollments(None, None, None)?;
    dbg!(enrollments);

    Ok(())
}
