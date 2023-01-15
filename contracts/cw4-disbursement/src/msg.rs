use cosmwasm_schema::{cw_serde, QueryResponses};
use cw20::Cw20ReceiveMsg;
use cw4::{Member, MemberListResponse, MemberResponse, TotalWeightResponse};
use cw4_group::msg::ExecuteMsg as Cw4GroupExecuteMsg;
use cw721::Cw721ReceiveMsg;
use cw_controllers::{AdminResponse, HooksResponse};
use cw_disbursement::{CwDisbursementExecuteMsg, DisbursementDataResponse};
use dao_macros::voting_module_query;

use crate::model::DumpStateResponse;

#[cw_serde]
pub struct InstantiateMsg {
    pub members: Vec<Member>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Cw4GroupExecute(Cw4GroupExecuteMsg),
    CwDisbursementExecute(CwDisbursementExecuteMsg),
    Receive(Cw20ReceiveMsg),
    ReceiveNft(Cw721ReceiveMsg),
}

#[derive(QueryResponses)]
#[voting_module_query]
#[cw_serde]
pub enum QueryMsg {
    #[returns(AdminResponse)]
    Admin {},
    #[returns(TotalWeightResponse)]
    TotalWeight { at_height: Option<u64> },
    #[returns(MemberListResponse)]
    ListMembers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(DumpStateResponse)]
    DumpState {},
    #[returns(u64)]
    LastUpdated {},
    #[returns(MemberResponse)]
    Member {
        addr: String,
        at_height: Option<u64>,
    },
    /// Shows all registered hooks.
    #[returns(HooksResponse)]
    Hooks {},
    #[returns(DisbursementDataResponse)]
    DisbursementData { key: Option<String> },
}

#[cw_serde]
pub struct MigrateMsg {}
