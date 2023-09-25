use cosmwasm_std::to_binary;
use dao_interface::state::ModuleInstantiateInfo;
use serde::Serialize;

pub fn get_competition_dao_instantiate_msg<T: Serialize>(
    cw4_id: u64,
    cw4_voting_module_id: u64,
    proposal_module_id: u64,
    proposal_module_instantiate: T,
    initial_members: Vec<cw4::Member>,
) -> dao_interface::msg::InstantiateMsg {
    dao_interface::msg::InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: cw4_voting_module_id,
            msg: to_binary(&dao_voting_cw4::msg::InstantiateMsg {
                cw4_group_code_id: cw4_id,
                initial_members,
            })
            .unwrap(),
            admin: None,
            label: "DAO DAO voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: proposal_module_id,
            msg: to_binary(&proposal_module_instantiate).unwrap(),
            admin: None,
            label: "DAO DAO governance module".to_string(),
        }],
        initial_items: None,
    }
}
