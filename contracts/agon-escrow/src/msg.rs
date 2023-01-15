use cosmwasm_schema::{cw_serde, QueryResponses};
use cw20::Cw20ReceiveMsg;
use cw721::Cw721ReceiveMsg;
use cw_competition::{CwCompetitionResultMsg, CwCompetitionStateChangedMsg};
use cw_disbursement::MemberBalance;
use cw_tokens::GenericTokenBalance;

#[cw_serde]
pub struct InstantiateMsg {
    pub arbiter: Option<String>,
    pub due: Vec<MemberBalance>,
    pub stake: Vec<MemberBalance>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Refund {},
    ReceiveNative {},
    Receive(Cw20ReceiveMsg),
    ReceiveNft(Cw721ReceiveMsg),
    HandleCompetitionResult(CwCompetitionResultMsg),
    HandleCompetitionStateChanged(CwCompetitionStateChangedMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<GenericTokenBalance>)]
    Balance { member: String },
    #[returns(Vec<GenericTokenBalance>)]
    Due { member: String },
    #[returns(Vec<GenericTokenBalance>)]
    Total {},
}

#[cw_serde]
pub struct MigrateMsg {}
