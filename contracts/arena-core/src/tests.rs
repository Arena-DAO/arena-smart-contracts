use cosmwasm_std::{to_binary, Addr, Decimal, Empty, Uint128, WasmMsg};
use cw20::Cw20Coin;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use dao_interface::{
    query::GetItemResponse,
    state::{Admin, ModuleInstantiateInfo, ProposalModule},
};
use dao_testing::helpers::instantiate_with_cw4_groups_governance;
use dao_voting::proposal::SingleChoiceProposeMsg;

use crate::msg::{InstantiateExt, InstantiateMsg};

const CREATOR: &str = "ismellike";

fn sudo_proposal_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        dao_proposal_sudo::contract::execute,
        dao_proposal_sudo::contract::instantiate,
        dao_proposal_sudo::contract::query,
    );
    Box::new(contract)
}

#[test]
fn attach_arena_core() {
    let mut app = App::default();
    let gov_id = app.store_code(dao_testing::contracts::dao_dao_contract());
    let govmod_id = app.store_code(sudo_proposal_contract());
    let dpm_id = app.store_code(arena_testing::contracts::dao_proposal_multiple_contract());
    let arena_id = app.store_code(arena_testing::contracts::arena_dao_core_contract());
    let wager_module_id = app.store_code(arena_testing::contracts::arena_wager_module_contract());

    let govmod_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR.to_owned(),
    };
    let gov_instantiate = dao_interface::msg::InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "Arena DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            label: "voting module".to_string(),
        }],
        initial_items: None,
    };

    let gov_addr = app
        .instantiate_contract(
            gov_id,
            Addr::unchecked(CREATOR),
            &gov_instantiate,
            &[],
            "cw-governance",
            None,
        )
        .unwrap();

    let proposal_module = &app
        .wrap()
        .query_wasm_smart::<Vec<ProposalModule>>(
            gov_addr.clone(),
            &dao_interface::msg::QueryMsg::ProposalModules {
                start_after: None,
                limit: Some(1u32),
            },
        )
        .unwrap()[0];

    let res = app.execute_contract(
        Addr::unchecked(CREATOR),
        proposal_module.address.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: gov_addr.to_string(),
                funds: vec![],
                msg: to_binary(&dao_interface::msg::ExecuteMsg::UpdateProposalModules {
                    to_add: vec![ModuleInstantiateInfo {
                        code_id: dpm_id,
                        msg: to_binary(&dao_proposal_multiple::msg::InstantiateMsg {
                            voting_strategy:
                                dao_voting::multiple_choice::VotingStrategy::SingleChoice {
                                    quorum: dao_voting::threshold::PercentageThreshold::Majority {},
                                },
                            min_voting_period: None,
                            max_voting_period: cw_utils::Duration::Time(1209600u64),
                            only_members_execute: false,
                            allow_revoting: false,
                            pre_propose_info:
                                dao_voting::pre_propose::PreProposeInfo::ModuleMayPropose {
                                    info: ModuleInstantiateInfo {
                                        code_id: arena_id,
                                        msg: to_binary(&InstantiateMsg {
                                            deposit_info: None,
                                            open_proposal_submission: false,
                                            extension: InstantiateExt {
                                                competition_modules_instantiate_info: vec![
                                                    ModuleInstantiateInfo {
                                                        code_id: wager_module_id,
                                                        msg: to_binary(&arena_wager_module::msg::InstantiateMsg {
                                                            key: "wagers".to_string(), 
                                                            description: "The Arena Protocol Wager Module".to_string(), 
                                                            extension: Empty {}}).unwrap(),
                                                        admin: Some(Admin::CoreModule {}),
                                                        label: "Arena Wager Module".to_string(),
                                                    },
                                                ],
                                                rulesets: vec![],
                                                tax: Decimal::new(Uint128::from(
                                                    150000000000000000u128,
                                                )),
                                            },
                                        })
                                        .unwrap(),
                                        admin: Some(Admin::CoreModule {
                                        }),
                                        label: "Arena Core".to_string(),
                                    },
                                },
                            close_proposal_on_execution_failure: true,
                        })
                        .unwrap(),
                        admin: Some(Admin::Address { addr: gov_addr.to_string() }),
                        label: "Proposal Multiple".to_string(),
                    }],
                    to_disable: vec![],
                })
                .unwrap(),
            }
            .into()],
        },
        &vec![],
    );

    assert!(res.is_ok());

    let item_response: GetItemResponse = app
        .wrap()
        .query_wasm_smart(
            gov_addr,
            &dao_interface::msg::QueryMsg::GetItem {
                key: crate::contract::ITEM_KEY.to_owned(),
            },
        )
        .unwrap();

    assert!(item_response.item.is_some());
}

#[test]
fn attach_arena_core_proposal() {
    let mut app = App::default();
    let proposal_id = app.store_code(dao_testing::contracts::proposal_single_contract());
    let dpm_id = app.store_code(arena_testing::contracts::dao_proposal_multiple_contract());
    let arena_id = app.store_code(arena_testing::contracts::arena_dao_core_contract());
    let wager_module_id = app.store_code(arena_testing::contracts::arena_wager_module_contract());

    let gov_addr = instantiate_with_cw4_groups_governance(
        &mut app,
        proposal_id,
        to_binary(&dao_proposal_single::msg::InstantiateMsg {
            threshold: {
                dao_voting::threshold::Threshold::AbsolutePercentage {
                    percentage: dao_voting::threshold::PercentageThreshold::Majority {},
                }
            },
            max_voting_period: { cw_utils::Duration::Time(1000000000u64) },
            min_voting_period: None,
            only_members_execute: false,
            allow_revoting: false,
            pre_propose_info: { dao_voting::pre_propose::PreProposeInfo::AnyoneMayPropose {} },
            close_proposal_on_execution_failure: false,
        })
        .unwrap(),
        Some(vec![Cw20Coin {
            address: CREATOR.to_string(),
            amount: Uint128::one(),
        }]),
    );

    let res = app.wrap().query_wasm_smart::<Vec<ProposalModule>>(
        gov_addr.clone(),
        &dao_interface::msg::QueryMsg::ProposalModules {
            start_after: None,
            limit: Some(1u32),
        },
    );

    assert!(res.is_ok());
    let proposal_module = &res.unwrap()[0];

    let proposal_msg = dao_proposal_single::msg::ExecuteMsg::Propose(SingleChoiceProposeMsg {
        title: "Enable the Arena Proposal Module".to_string(),
        description: "Decentralized Competition".to_string(),
        msgs: vec![WasmMsg::Execute {
            contract_addr: gov_addr.to_string(),
            funds: vec![],
            msg: to_binary(&dao_interface::msg::ExecuteMsg::UpdateProposalModules {
                to_add: vec![ModuleInstantiateInfo {
                    code_id: dpm_id,
                    msg: to_binary(&dao_proposal_multiple::msg::InstantiateMsg {
                        voting_strategy:
                            dao_voting::multiple_choice::VotingStrategy::SingleChoice {
                                quorum: dao_voting::threshold::PercentageThreshold::Majority {},
                            },
                        min_voting_period: None,
                        max_voting_period: cw_utils::Duration::Time(1209600u64),
                        only_members_execute: false,
                        allow_revoting: false,
                        pre_propose_info:
                            dao_voting::pre_propose::PreProposeInfo::ModuleMayPropose {
                                info: ModuleInstantiateInfo {
                                    code_id: arena_id,
                                    msg: to_binary(&InstantiateMsg {
                                        deposit_info: None,
                                        open_proposal_submission: false,
                                        extension: InstantiateExt {
                                            competition_modules_instantiate_info: vec![
                                                ModuleInstantiateInfo {
                                                    code_id: wager_module_id,
                                                    msg: to_binary(
                                                        &arena_wager_module::msg::InstantiateMsg {
                                                            key: "wagers".to_string(),
                                                            description:
                                                                "The Arena Protocol Wager Module"
                                                                    .to_string(),
                                                            extension: Empty {},
                                                        },
                                                    )
                                                    .unwrap(),
                                                    admin: Some(Admin::CoreModule {}),
                                                    label: "Arena Wager Module".to_string(),
                                                },
                                            ],
                                            rulesets: vec![],
                                            tax: Decimal::new(Uint128::from(
                                                150000000000000000u128,
                                            )),
                                        },
                                    })
                                    .unwrap(),
                                    admin: Some(Admin::CoreModule {}),
                                    label: "Arena Core".to_string(),
                                },
                            },
                        close_proposal_on_execution_failure: true,
                    })
                    .unwrap(),
                    admin: Some(Admin::Address {
                        addr: gov_addr.to_string(),
                    }),
                    label: "Proposal Multiple".to_string(),
                }],
                to_disable: vec![],
            })
            .unwrap(),
        }
        .into()],
        proposer: None,
    });

    let res = app.execute_contract(
        Addr::unchecked(CREATOR),
        proposal_module.address.clone(),
        &proposal_msg,
        &vec![],
    );

    assert!(res.is_ok());

    let res = app.execute_contract(
        Addr::unchecked(CREATOR),
        proposal_module.address.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Vote {
            proposal_id: 1u64,
            vote: dao_voting::voting::Vote::Yes,
            rationale: None,
        },
        &vec![],
    );
    assert!(res.is_ok());

    let item_response: GetItemResponse = app
        .wrap()
        .query_wasm_smart(
            gov_addr.clone(),
            &dao_interface::msg::QueryMsg::GetItem {
                key: crate::contract::ITEM_KEY.to_owned(),
            },
        )
        .unwrap();

    assert!(item_response.item.is_none());

    let res = app.execute_contract(
        Addr::unchecked(CREATOR),
        proposal_module.address.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Execute { proposal_id: 1u64 },
        &vec![],
    );
    assert!(res.is_ok());

    let item_response: GetItemResponse = app
        .wrap()
        .query_wasm_smart(
            gov_addr,
            &dao_interface::msg::QueryMsg::GetItem {
                key: crate::contract::ITEM_KEY.to_owned(),
            },
        )
        .unwrap();

    assert!(item_response.item.is_some());
}
