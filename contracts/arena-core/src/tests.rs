use crate::{
    msg::{InstantiateExt, InstantiateMsg},
    state::Ruleset,
};
use cosmwasm_std::{to_binary, Addr, CosmosMsg, Decimal, Uint128, WasmMsg};
use cw20::Cw20Coin;
use cw_multi_test::{App, Executor};
use dao_interface::ModuleInstantiateInfo;
use dao_testing::helpers::instantiate_with_cw4_groups_governance;

const MAIN_ADDR: &str = "main";

struct Context {
    dao: Addr,
    spm_addr: Addr,
    arena_dao_module: Addr,
}

fn create_context(app: &mut App) -> Context {
    let proposal_single_code_id =
        app.store_code(dao_testing::contracts::proposal_single_contract());

    let dao = instantiate_with_cw4_groups_governance(
        app,
        proposal_single_code_id,
        to_binary(&dao_proposal_single::msg::InstantiateMsg {
            threshold: dao_voting::threshold::Threshold::AbsolutePercentage {
                percentage: dao_voting::threshold::PercentageThreshold::Majority {},
            },
            max_voting_period: cw_utils::Duration::Time(100000u64),
            min_voting_period: None,
            only_members_execute: false,
            allow_revoting: false,
            pre_propose_info: dao_voting::pre_propose::PreProposeInfo::AnyoneMayPropose {},
            close_proposal_on_execution_failure: true,
        })
        .unwrap(),
        Some(vec![Cw20Coin {
            address: MAIN_ADDR.to_string(),
            amount: Uint128::one(),
        }]),
    );

    let proposal_multiple_id =
        app.store_code(arena_testing::contracts::dao_proposal_multiple_contract());
    let arena_dao_code_id = app.store_code(arena_testing::contracts::arena_dao_core_contract());

    let proposal_modules = get_active_modules(app, dao.clone());
    assert_eq!(proposal_modules.len(), 1);

    let spm_addr = proposal_modules.into_iter().nth(0).unwrap().address.clone();
    app.execute_contract(
        Addr::unchecked(MAIN_ADDR),
        spm_addr.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Propose(
            dao_voting::proposal::SingleChoiceProposeMsg {
                title: "Arena Proposal Module".to_string(),
                description: "Enable Decentralized Competition!".to_string(),
                msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: dao.to_string(),
                    msg: to_binary(&dao_core::msg::ExecuteMsg::UpdateProposalModules {
                        to_add: vec![ModuleInstantiateInfo {
                            code_id: proposal_multiple_id,
                            msg: to_binary(&dao_proposal_multiple::msg::InstantiateMsg {
                                voting_strategy:
                                    dao_voting::multiple_choice::VotingStrategy::SingleChoice {
                                        quorum:
                                            dao_voting::threshold::PercentageThreshold::Majority {},
                                    },
                                min_voting_period: None,
                                max_voting_period: cw_utils::Duration::Time(100000u64),
                                only_members_execute: false,
                                allow_revoting: false,
                                pre_propose_info:
                                    dao_voting::pre_propose::PreProposeInfo::ModuleMayPropose {
                                        info: ModuleInstantiateInfo {
                                            code_id: arena_dao_code_id,
                                            msg: to_binary(&InstantiateMsg {
                                                deposit_info: None,
                                                open_proposal_submission: true,
                                                extension: InstantiateExt {
                                                    competition_modules_instantiate_info: vec![],
                                                    rulesets: vec![Ruleset {
                                                        rules: vec!["Rule 1".to_string()],
                                                        description: "Test title".to_string(),
                                                        is_enabled: true,
                                                    }],
                                                    tax: Decimal::percent(15u64),
                                                },
                                            })
                                            .unwrap(),
                                            admin: None,
                                            label: "Arena Core".to_string(),
                                        },
                                    },
                                close_proposal_on_execution_failure: true,
                            })
                            .unwrap(),
                            admin: None,
                            label: "Arena Core".to_string(),
                        }],
                        to_disable: vec![],
                    })
                    .unwrap(),
                    funds: vec![],
                })],
                proposer: None,
            },
        ),
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(MAIN_ADDR),
        spm_addr.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Vote {
            proposal_id: 1u64,
            vote: dao_voting::voting::Vote::Yes,
            rationale: None,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(MAIN_ADDR),
        spm_addr.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Execute { proposal_id: 1u64 },
        &[],
    )
    .unwrap();

    let proposal_modules = get_active_modules(app, dao.clone());
    assert_eq!(proposal_modules.len(), 1);

    Context {
        dao,
        spm_addr,
        arena_dao_module: Addr::unchecked("test"),
    }
}

fn get_active_modules(app: &App, dao_core: Addr) -> Vec<dao_core::state::ProposalModule> {
    app.wrap()
        .query_wasm_smart(
            dao_core,
            &dao_core::msg::QueryMsg::ActiveProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap()
}

#[test]
fn instantiate() {
    let mut app = App::default();
    let context = create_context(&mut app);
}

#[test]
fn cannot_propose() {}
