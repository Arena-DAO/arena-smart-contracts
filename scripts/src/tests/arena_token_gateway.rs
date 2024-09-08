use crate::tests::helpers::{setup_arena, setup_vesting, setup_voting_module};
use arena_token_gateway::msg::{ApplyMsg, ExecuteMsg, ExecuteMsgFns as _, QueryMsgFns as _};
use cosmwasm_std::{coins, to_json_binary, CosmosMsg, Decimal, Uint128, WasmMsg};
use cw4::Member;
use cw_orch::{anyhow, prelude::*};
use cw_payroll_factory::msg::QueryMsgFns as _;
use dao_voting::{proposal::SingleChoiceProposeMsg, voting::SingleChoiceAutoVote};

use super::{DENOM, PREFIX};

#[test]
fn test_instantiate_arena_token_gateway() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (arena, admin) = setup_arena(&mock)?;
    setup_vesting(&arena, mock.block_info()?.chain_id, &admin)?;

    // Instantiate the arena_token_gateway
    arena.arena_token_gateway.instantiate(
        &arena_token_gateway::msg::InstantiateMsg {
            owner: arena.dao_dao.dao_core.addr_str()?,
            config: arena_token_gateway::state::VestingConfiguration {
                upfront_ratio: Decimal::percent(10),
                vesting_time: 31_536_000, // 1 year in seconds
                denom: DENOM.to_string(),
            },
        },
        Some(&arena.dao_dao.dao_core.address()?),
        None,
    )?;

    // Query the vesting configuration
    let vesting_config = arena.arena_token_gateway.vesting_configuration()?;

    assert_eq!(vesting_config.upfront_ratio, Decimal::percent(10));
    assert_eq!(vesting_config.vesting_time, 31_536_000);
    assert_eq!(vesting_config.denom, DENOM);

    Ok(())
}

#[test]
fn test_apply_and_accept_application() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;
    setup_vesting(&arena, mock.block_info()?.chain_id, &admin)?;
    setup_voting_module(
        &mock,
        &arena,
        vec![Member {
            addr: admin.to_string(),
            weight: 1,
        }],
    )?;

    mock.add_balance(
        &arena.dao_dao.dao_core.address()?,
        coins(1_000_000_000_000, DENOM),
    )?;

    // Instantiate the arena_token_gateway (similar to previous test)
    arena.arena_token_gateway.instantiate(
        &arena_token_gateway::msg::InstantiateMsg {
            owner: arena.dao_dao.dao_core.addr_str()?,
            config: arena_token_gateway::state::VestingConfiguration {
                upfront_ratio: Decimal::percent(10),
                vesting_time: 31_536_000,
                denom: DENOM.to_string(),
            },
        },
        Some(&arena.dao_dao.dao_core.address()?),
        None,
    )?;

    let applicant = mock.addr_make("applicant");
    arena.arena_token_gateway.set_sender(&applicant);

    // Apply for tokens
    arena.arena_token_gateway.apply(ApplyMsg {
        title: "Test Application".to_string(),
        description: "This is a test application".to_string(),
        requested_amount: Uint128::new(1000000),
        project_links: vec![arena_token_gateway::state::ProjectLink {
            title: "GitHub".to_string(),
            url: "https://github.com/test/project".to_string(),
        }],
    })?;

    // Query the application
    let application = arena.arena_token_gateway.application(1u128)?;

    assert_eq!(application.application.title, "Test Application");
    assert_eq!(
        application.application.status,
        arena_token_gateway::state::ApplicationStatus::Pending {}
    );

    // Accept the application with the upfront amount attached
    arena.dao_dao.dao_proposal_single.call_as(&admin).execute(
        &dao_proposal_single::msg::ExecuteMsg::Propose(SingleChoiceProposeMsg {
            title: "Accept token gateway application".to_owned(),
            description: "Testing the token gateway application process with callback messages"
                .to_owned(),
            msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: arena.arena_token_gateway.addr_str()?,
                msg: to_json_binary(&ExecuteMsg::AcceptApplication {
                    application_id: Uint128::one(),
                })?,
                funds: coins(100000, DENOM),
            })],
            proposer: None,
            vote: Some(SingleChoiceAutoVote {
                vote: dao_voting::voting::Vote::Yes,
                rationale: None,
            }),
        }),
        None,
    )?;
    let res = arena.dao_dao.dao_proposal_single.execute(
        &dao_proposal_single::msg::ExecuteMsg::Execute { proposal_id: 1 },
        None,
    )?;
    dbg!(res.events);
    dbg!(arena.dao_dao.cw_payroll_factory.address()?);

    // Query the application again
    let updated_application = arena.arena_token_gateway.application(1u128)?;

    assert_eq!(
        updated_application.application.status,
        arena_token_gateway::state::ApplicationStatus::Accepted {}
    );

    // Ensure the upfront amount was received
    let balance = mock.query_balance(&applicant, DENOM)?;
    assert_eq!(balance, Uint128::new(100000));

    // Ensure vesting was successful
    let payroll_address = arena.arena_token_gateway.payroll_address()?;

    arena
        .dao_dao
        .cw_payroll_factory
        .set_address(&payroll_address);

    let vesting_contracts = arena
        .dao_dao
        .cw_payroll_factory
        .list_vesting_contracts(None, None)?;

    assert!(!vesting_contracts.is_empty());

    // Ensure token gateway balance is empty
    let gateway_balance = mock.query_balance(&arena.arena_token_gateway.address()?, DENOM)?;
    assert!(gateway_balance.is_zero());

    Ok(())
}

#[test]
fn test_reject_application() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;
    setup_vesting(&arena, mock.block_info()?.chain_id, &admin)?;

    // Instantiate the arena_token_gateway (similar to previous test)
    arena.arena_token_gateway.instantiate(
        &arena_token_gateway::msg::InstantiateMsg {
            owner: arena.dao_dao.dao_core.addr_str()?,
            config: arena_token_gateway::state::VestingConfiguration {
                upfront_ratio: Decimal::percent(10),
                vesting_time: 31_536_000,
                denom: DENOM.to_string(),
            },
        },
        Some(&arena.dao_dao.dao_core.address()?),
        None,
    )?;

    let applicant = mock.addr_make("applicant");
    arena.arena_token_gateway.set_sender(&applicant);

    // Apply for tokens
    arena.arena_token_gateway.apply(ApplyMsg {
        title: "Test Application".to_string(),
        description: "This is a test application".to_string(),
        requested_amount: Uint128::new(1000000),
        project_links: vec![arena_token_gateway::state::ProjectLink {
            title: "GitHub".to_string(),
            url: "https://github.com/test/project".to_string(),
        }],
    })?;

    // Reject the application
    arena
        .arena_token_gateway
        .set_sender(&arena.dao_dao.dao_core.address()?);
    arena
        .arena_token_gateway
        .reject_application(1u128, Some("Not eligible".to_string()))?;

    // Query the application
    let rejected_application = arena.arena_token_gateway.application(1u128)?;

    assert_eq!(
        rejected_application.application.status,
        arena_token_gateway::state::ApplicationStatus::Rejected {
            reason: Some("Not eligible".to_string())
        }
    );

    Ok(())
}

#[test]
fn test_update_vesting_configuration() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;
    setup_vesting(&arena, mock.block_info()?.chain_id, &admin)?;

    // Instantiate the arena_token_gateway (similar to previous tests)
    arena.arena_token_gateway.instantiate(
        &arena_token_gateway::msg::InstantiateMsg {
            owner: arena.dao_dao.dao_core.addr_str()?,
            config: arena_token_gateway::state::VestingConfiguration {
                upfront_ratio: Decimal::percent(10),
                vesting_time: 31_536_000,
                denom: DENOM.to_string(),
            },
        },
        Some(&arena.dao_dao.dao_core.address()?),
        None,
    )?;

    // Update the vesting configuration
    arena
        .arena_token_gateway
        .set_sender(&arena.dao_dao.dao_core.address()?);
    arena.arena_token_gateway.update_vesting_configuration(
        arena_token_gateway::state::VestingConfiguration {
            upfront_ratio: Decimal::percent(20),
            vesting_time: 15_768_000, // 6 months
            denom: DENOM.to_string(),
        },
    )?;

    // Query the updated vesting configuration
    let updated_config = arena.arena_token_gateway.vesting_configuration()?;

    assert_eq!(updated_config.upfront_ratio, Decimal::percent(20));
    assert_eq!(updated_config.vesting_time, 15_768_000);
    assert_eq!(updated_config.denom, DENOM);

    Ok(())
}

#[test]
fn test_withdraw_application() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;
    setup_vesting(&arena, mock.block_info()?.chain_id, &admin)?;

    // Instantiate the arena_token_gateway (similar to previous tests)
    arena.arena_token_gateway.instantiate(
        &arena_token_gateway::msg::InstantiateMsg {
            owner: arena.dao_dao.dao_core.addr_str()?,
            config: arena_token_gateway::state::VestingConfiguration {
                upfront_ratio: Decimal::percent(10),
                vesting_time: 31_536_000,
                denom: DENOM.to_string(),
            },
        },
        Some(&arena.dao_dao.dao_core.address()?),
        None,
    )?;

    let applicant = mock.addr_make("applicant");
    arena.arena_token_gateway.set_sender(&applicant);

    // Apply for tokens
    arena.arena_token_gateway.apply(ApplyMsg {
        title: "Test Application".to_string(),
        description: "This is a test application".to_string(),
        requested_amount: Uint128::new(1000000),
        project_links: vec![arena_token_gateway::state::ProjectLink {
            title: "GitHub".to_string(),
            url: "https://github.com/test/project".to_string(),
        }],
    })?;

    // Withdraw the application
    arena.arena_token_gateway.withdraw(1u128)?;

    // Try to query the application (should fail)
    let result = arena.arena_token_gateway.application(1u128);
    assert!(result.is_err());

    Ok(())
}
