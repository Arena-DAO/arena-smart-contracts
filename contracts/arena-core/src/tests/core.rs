use arena_interface::{
    core::{
        CompetitionCategory, EditCompetitionCategory, InstantiateExt, InstantiateMsg,
        NewCompetitionCategory, NewRuleset, Ruleset,
    },
    fees::TaxConfiguration,
};
use cosmwasm_std::{to_json_binary, Addr, Decimal, Empty, Uint128, WasmMsg};
use cw4::Member;
use cw_multi_test::{
    next_block, App, AppResponse, BankKeeper, Contract, ContractWrapper, Executor, MockApiBech32,
};
use cw_utils::Duration;
use dao_interface::{
    query::GetItemResponse,
    state::{Admin, ModuleInstantiateInfo, ProposalModule},
};

use crate::tests::app::get_app;

pub const ADMIN: &str = "ismellike";

pub fn get_attr_value(response: &AppResponse, key: &str) -> Option<String> {
    for event in &response.events {
        for attribute in &event.attributes {
            if attribute.key == key {
                return Some(attribute.value.clone());
            }
        }
    }
    None
}

pub fn sudo_proposal_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        dao_proposal_sudo::contract::execute,
        dao_proposal_sudo::contract::instantiate,
        dao_proposal_sudo::contract::query,
    );
    Box::new(contract)
}

pub struct CoreContext {
    pub dao_addr: Addr,
    pub arena_core_addr: Addr,
    pub proposal_module_addr: Addr,
    pub sudo_proposal_addr: Addr,
}

pub fn setup_core_context(
    app: &mut App<BankKeeper, MockApiBech32>,
    members: Vec<Member>,
) -> CoreContext {
    let dao_proposal_single_id =
        app.store_code(arena_testing::contracts::proposal_single_contract());
    let arena_core_id = app.store_code(arena_testing::contracts::arena_dao_core_contract());
    let dao_proposal_sudo_id = app.store_code(sudo_proposal_contract());
    let dao_core_id = app.store_code(arena_testing::contracts::dao_dao_contract());
    let cw4_id = app.store_code(arena_testing::contracts::cw4_group_contract());
    let cw4_voting_module_id = app.store_code(arena_testing::contracts::dao_voting_cw4_contract());
    let admin = app.api().addr_make(ADMIN);

    // Create the DAO
    let sudo_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
        root: admin.to_string(),
    };

    let gov_instantiate = dao_interface::msg::InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "Arena DAO".to_string(),
        description: "Decentralized Competition".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: cw4_voting_module_id,
            msg: to_json_binary(&dao_voting_cw4::msg::InstantiateMsg {
                group_contract: dao_voting_cw4::msg::GroupContract::New {
                    cw4_group_code_id: cw4_id,
                    initial_members: members,
                },
            })
            .unwrap(),
            admin: None,
            label: "voting module".to_string(),
            funds: vec![],
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: dao_proposal_sudo_id,
            msg: to_json_binary(&sudo_instantiate).unwrap(),
            admin: None,
            label: "voting module".to_string(),
            funds: vec![],
        }],
        initial_items: None,
    };

    let result = app.instantiate_contract(
        dao_core_id,
        admin.clone(),
        &gov_instantiate,
        &[],
        "cw-governance",
        None,
    );

    assert!(result.is_ok());
    let dao_addr = result.unwrap();

    // Query for the sudo proposal module
    let result = app.wrap().query_wasm_smart::<Vec<ProposalModule>>(
        dao_addr.clone(),
        &dao_interface::msg::QueryMsg::ProposalModules {
            start_after: None,
            limit: Some(1u32),
        },
    );
    assert!(result.is_ok());
    assert!(!result.as_ref().unwrap().is_empty());

    let proposal_module = &result.unwrap()[0];

    // Attach the arena-core extension
    let result = app.execute_contract(
        admin.clone(),
        proposal_module.address.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: dao_addr.to_string(),
                funds: vec![],
                msg: to_json_binary(&dao_interface::msg::ExecuteMsg::UpdateProposalModules {
                    to_add: vec![ModuleInstantiateInfo {
                        code_id: dao_proposal_single_id,
                        msg: to_json_binary(&dao_proposal_single::msg::InstantiateMsg {
                            threshold: dao_voting::threshold::Threshold::AbsolutePercentage {
                                percentage: dao_voting::threshold::PercentageThreshold::Majority {},
                            },
                            min_voting_period: None,
                            max_voting_period: Duration::Time(1209600u64),
                            only_members_execute: false,
                            allow_revoting: false,
                            veto: None,
                            pre_propose_info:
                                dao_voting::pre_propose::PreProposeInfo::ModuleMayPropose {
                                    info: ModuleInstantiateInfo {
                                        code_id: arena_core_id,
                                        msg: to_json_binary(&InstantiateMsg {
                                            deposit_info: None,
                                            submission_policy: dao_voting::pre_propose::PreProposeSubmissionPolicy::Specific { dao_members: true, allowlist: vec![], denylist: vec![] },
                                            extension: InstantiateExt {
                                                competition_modules_instantiate_info: None,
                                                categories: Some(vec![NewCompetitionCategory {
                                                    name: "Test Category".to_string(),
                                                }]),
                                                rulesets: Some(vec![
                                                    NewRuleset {
                                                        category_id: Uint128::one(),
                                                        rules: vec![
                                                            "This is a rule".to_string(),
                                                            "This is another rule".to_string(),
                                                        ],
                                                        description: "Test Ruleset 1".to_string(),
                                                    },
                                                    NewRuleset {
                                                        category_id: Uint128::one(),
                                                        rules: vec![
                                                            "This is a rule".to_string(),
                                                            "This is another rule".to_string(),
                                                        ],
                                                        description: "Test Ruleset 2".to_string(),
                                                    },
                                                ]),
                                                tax: Decimal::new(Uint128::from(
                                                    150000000000000000u128,
                                                )),
                                                tax_configuration: TaxConfiguration {
                                                    cw20_msg: None,
                                                    cw721_msg: None,
                                                },
                                                rating_period: Duration::Height(10u64),
                                            },
                                        })
                                        .unwrap(),
                                        admin: None,
                                        label: "Arena Core".to_string(),
                                        funds: vec![],
                                    },
                                },
                            close_proposal_on_execution_failure: true,
                        })
                        .unwrap(),
                        admin: Some(Admin::Address {
                            addr: dao_addr.to_string(),
                        }),
                        label: "Proposal Multiple".to_string(),
                        funds: vec![],
                    }],
                    to_disable: vec![],
                })
                .unwrap(),
            }
            .into()],
        },
        &[],
    );
    assert!(result.is_ok());

    // Get the proposal module addr from the response
    let maybe_val = get_attr_value(result.as_ref().unwrap(), "prop_module");
    assert!(maybe_val.is_some());
    let proposal_module_addr = Addr::unchecked(maybe_val.unwrap());

    // Get Arena Core addr from the DAO's GetItem query
    let item_response: GetItemResponse = app
        .wrap()
        .query_wasm_smart(
            dao_addr.clone(),
            &dao_interface::msg::QueryMsg::GetItem {
                key: crate::contract::ITEM_KEY.to_owned(),
            },
        )
        .unwrap();
    assert!(item_response.item.is_some());
    let arena_core_addr = Addr::unchecked(item_response.item.unwrap());

    // Update the block
    app.update_block(next_block);

    CoreContext {
        dao_addr,
        arena_core_addr,
        proposal_module_addr,
        sudo_proposal_addr: proposal_module.address.clone(),
    }
}

#[test]
pub fn test_categories() {
    let mut app = get_app();

    let admin = app.api().addr_make(ADMIN);
    let context = setup_core_context(
        &mut app,
        vec![Member {
            addr: admin.to_string(),
            weight: 1u64,
        }],
    );

    // Test adding a new category and disabling the original category
    let result = app.execute_contract(
        admin.clone(),
        context.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: context.arena_core_addr.to_string(),
                funds: vec![],
                msg: to_json_binary(&arena_interface::core::ExecuteMsg::Extension {
                    msg: arena_interface::core::ExecuteExt::UpdateCategories {
                        to_add: Some(vec![NewCompetitionCategory {
                            name: "New Category".to_string(),
                        }]),
                        to_edit: Some(vec![EditCompetitionCategory::Disable {
                            category_id: Uint128::one(),
                        }]),
                    },
                })
                .unwrap(),
            }
            .into()],
        },
        &[],
    );
    assert!(result.is_ok());

    // Test querying categories
    let categories: Vec<CompetitionCategory> = app
        .wrap()
        .query_wasm_smart(
            context.arena_core_addr.clone(),
            &arena_interface::core::QueryMsg::QueryExtension {
                msg: arena_interface::core::QueryExt::Categories {
                    start_after: None,
                    limit: None,
                    include_disabled: None,
                },
            },
        )
        .unwrap();
    assert_eq!(categories.len(), 1);
    assert_eq!(categories[0].name, "New Category");
    assert!(categories[0].is_enabled);

    // Test querying disabled categories
    let categories: Vec<CompetitionCategory> = app
        .wrap()
        .query_wasm_smart(
            context.arena_core_addr.clone(),
            &arena_interface::core::QueryMsg::QueryExtension {
                msg: arena_interface::core::QueryExt::Categories {
                    start_after: None,
                    limit: None,
                    include_disabled: Some(true),
                },
            },
        )
        .unwrap();
    assert_eq!(categories.len(), 2);
    assert_eq!(categories[0].name, "Test Category");
    assert!(!categories[0].is_enabled);
}

#[test]
pub fn test_rulesets() {
    let mut app = get_app();

    let admin = app.api().addr_make(ADMIN);
    let context = setup_core_context(
        &mut app,
        vec![Member {
            addr: admin.to_string(),
            weight: 1u64,
        }],
    );

    // Instantiate a new ruleset
    let result = app.execute_contract(
        admin.clone(),
        context.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: context.arena_core_addr.to_string(),
                funds: vec![],
                msg: to_json_binary(&arena_interface::core::ExecuteMsg::Extension {
                    msg: arena_interface::core::ExecuteExt::UpdateRulesets {
                        to_add: Some(vec![arena_interface::core::NewRuleset {
                            category_id: Uint128::one(),
                            rules: vec!["Rule 1".to_string(), "Rule 2".to_string()],
                            description: "Test Ruleset 3".to_string(),
                        }]),
                        to_disable: None,
                    },
                })
                .unwrap(),
            }
            .into()],
        },
        &[],
    );
    assert!(result.is_ok());

    // Query the ruleset
    let rulesets: Vec<Ruleset> = app
        .wrap()
        .query_wasm_smart(
            context.arena_core_addr.clone(),
            &arena_interface::core::QueryMsg::QueryExtension {
                msg: arena_interface::core::QueryExt::Rulesets {
                    category_id: Uint128::one(),
                    start_after: None,
                    limit: None,
                    include_disabled: None,
                },
            },
        )
        .unwrap();
    assert_eq!(rulesets.len(), 3);

    // Disable the ruleset
    let result = app.execute_contract(
        admin.clone(),
        context.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: context.arena_core_addr.to_string(),
                funds: vec![],
                msg: to_json_binary(&arena_interface::core::ExecuteMsg::Extension {
                    msg: arena_interface::core::ExecuteExt::UpdateRulesets {
                        to_add: None,
                        to_disable: Some(vec![Uint128::one()]),
                    },
                })
                .unwrap(),
            }
            .into()],
        },
        &[],
    );
    assert!(result.is_ok());

    // Query the ruleset again
    let rulesets: Vec<Ruleset> = app
        .wrap()
        .query_wasm_smart(
            context.arena_core_addr.clone(),
            &arena_interface::core::QueryMsg::QueryExtension {
                msg: arena_interface::core::QueryExt::Rulesets {
                    category_id: Uint128::one(),
                    start_after: None,
                    limit: None,
                    include_disabled: None,
                },
            },
        )
        .unwrap();
    assert_eq!(rulesets.len(), 2);

    let rulesets: Vec<Ruleset> = app
        .wrap()
        .query_wasm_smart(
            context.arena_core_addr.clone(),
            &arena_interface::core::QueryMsg::QueryExtension {
                msg: arena_interface::core::QueryExt::Rulesets {
                    category_id: Uint128::one(),
                    start_after: None,
                    limit: None,
                    include_disabled: Some(true),
                },
            },
        )
        .unwrap();
    assert_eq!(rulesets.len(), 3);

    // Try to add a ruleset for a category that does not exist
    let result = app.execute_contract(
        admin.clone(),
        context.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: context.arena_core_addr.to_string(),
                funds: vec![],
                msg: to_json_binary(&arena_interface::core::ExecuteMsg::Extension {
                    msg: arena_interface::core::ExecuteExt::UpdateRulesets {
                        to_add: Some(vec![arena_interface::core::NewRuleset {
                            category_id: Uint128::from(9999u128), // Non-existent category
                            rules: vec!["Rule 1".to_string(), "Rule 2".to_string()],
                            description: "Test Ruleset 4".to_string(),
                        }]),
                        to_disable: None,
                    },
                })
                .unwrap(),
            }
            .into()],
        },
        &[],
    );
    assert!(result.is_err());
}
