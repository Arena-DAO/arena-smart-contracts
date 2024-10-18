use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint64;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use dao_interface::state::ModuleInstantiateInfo;

#[cw_serde]
pub struct InstantiateMsg {
    pub members: Option<Vec<AddMemberMsg>>,
}

#[cw_ownable_execute]
#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
pub enum ExecuteMsg {
    UpdateMembers {
        to_add: Option<Vec<AddMemberMsg>>,
        to_update: Option<Vec<MemberMsg>>,
        to_remove: Option<Vec<String>>,
    },
}

#[cw_serde]
pub struct AddMemberMsg {
    pub addr: String,
    /// If None, then the seed will be set as the members count at the time of insertion
    pub seed: Option<Uint64>,
}

#[cw_serde]
pub struct MemberMsg {
    pub addr: String,
    pub seed: Uint64,
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
pub enum QueryMsg {
    #[returns(Vec<cosmwasm_std::Addr>)]
    Members {
        start_after: Option<(Uint64, String)>,
        limit: Option<u32>,
    },
    #[returns(cosmwasm_std::Uint64)]
    MembersCount {},
}

#[cw_serde]
pub enum MigrateMsg {
    FromCompatible {},
}

#[cw_serde]
pub enum GroupContractInfo {
    Existing { addr: String },
    New { info: ModuleInstantiateInfo },
}
