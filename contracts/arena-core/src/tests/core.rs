use arena_core_interface::msg::{InstantiateExt, InstantiateMsg, NewRuleset};
use cosmwasm_std::{to_binary, Addr, Decimal, Empty, Uint128, WasmMsg};
use cw4::Member;
use cw_multi_test::{next_block, App, AppResponse, Contract, ContractWrapper, Executor};
use dao_interface::{
    query::GetItemResponse,
    state::{Admin, ModuleInstantiateInfo, ProposalModule},
};

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
    pub dao_proposal_multiple_id: u64,
    pub arena_core_id: u64,
    pub dao_core_id: u64,
    pub dao_proposal_sudo_id: u64,
    pub cw4_id: u64,
    pub cw4_voting_module_id: u64,
    pub sudo_proposal_addr: Addr,
    pub dao_addr: Addr,
    pub arena_core_addr: Addr,
    pub proposal_module_addr: Addr,
}

pub fn setup_core_context(app: &mut App, members: Vec<Member>) -> CoreContext {
    let dao_proposal_multiple_id =
        app.store_code(arena_testing::contracts::dao_proposal_multiple_contract());
    let arena_core_id = app.store_code(arena_testing::contracts::arena_dao_core_contract());
    let dao_proposal_sudo_id = app.store_code(sudo_proposal_contract());
    let dao_core_id = app.store_code(dao_testing::contracts::dao_dao_contract());
    let cw4_id = app.store_code(dao_testing::contracts::cw4_group_contract());
    let cw4_voting_module_id = app.store_code(dao_testing::contracts::dao_voting_cw4_contract());

    // Create the DAO
    let sudo_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
        root: ADMIN.to_owned(),
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
            msg: to_binary(&dao_voting_cw4::msg::InstantiateMsg {
                cw4_group_code_id: cw4_id,
                initial_members: members,
            })
            .unwrap(),
            admin: None,
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: dao_proposal_sudo_id,
            msg: to_binary(&sudo_instantiate).unwrap(),
            admin: None,
            label: "voting module".to_string(),
        }],
        initial_items: None,
    };

    let result = app.instantiate_contract(
        dao_core_id,
        Addr::unchecked(ADMIN),
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
        Addr::unchecked(ADMIN),
        proposal_module.address.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: dao_addr.to_string(),
                funds: vec![],
                msg: to_binary(&dao_interface::msg::ExecuteMsg::UpdateProposalModules {
                    to_add: vec![ModuleInstantiateInfo {
                        code_id: dao_proposal_multiple_id,
                        msg: to_binary(&dao_proposal_multiple::msg::InstantiateMsg {
                            voting_strategy:
                                dao_voting::multiple_choice::VotingStrategy::SingleChoice {
                                    quorum: dao_voting::threshold::PercentageThreshold::Majority {},
                                },
                            min_voting_period: None,
                            max_voting_period: cw_utils_v16::Duration::Time(1209600u64),
                            only_members_execute: false,
                            allow_revoting: false,
                            pre_propose_info:
                                dao_voting::pre_propose::PreProposeInfo::ModuleMayPropose {
                                    info: ModuleInstantiateInfo {
                                        code_id: arena_core_id,
                                        msg: to_binary(&InstantiateMsg {
                                            deposit_info: None,
                                            open_proposal_submission: false,
                                            extension: InstantiateExt {
                                                competition_modules_instantiate_info: vec![],
                                                rulesets: vec![
                                                    NewRuleset {
                                                        rules: vec![
                                                            "This is a rule".to_string(),
                                                            "This is another rule".to_string(),
                                                        ],
                                                        description: "This is a description"
                                                            .to_string(),
                                                    },
                                                    NewRuleset {
                                                        rules: vec![
                                                            "This is a rule".to_string(),
                                                            "This is another rule".to_string(),
                                                        ],
                                                        description: "This is a description"
                                                            .to_string(),
                                                    },
                                                ],
                                                tax: Decimal::new(Uint128::from(
                                                    150000000000000000u128,
                                                )),
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
                        admin: Some(Admin::Address {
                            addr: dao_addr.to_string(),
                        }),
                        label: "Proposal Multiple".to_string(),
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
        dao_proposal_multiple_id,
        arena_core_id,
        dao_core_id,
        dao_proposal_sudo_id,
        cw4_id,
        cw4_voting_module_id,
        dao_addr,
        arena_core_addr,
        sudo_proposal_addr: proposal_module.address.clone(),
        proposal_module_addr,
    }
}
