use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Binary, CosmosMsg, StdResult, WasmMsg};
use cw_balance::MemberShare;

#[cw_serde]
pub struct CompetitionEscrowDistributeMsg {
    pub distribution: Vec<MemberShare<String>>,
    pub remainder_addr: String,
}

impl CompetitionEscrowDistributeMsg {
    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = CompetitionEscrowMsg::Distribute(self);
        to_binary(&msg)
    }

    /// creates a cosmos_msg sending this struct to the named contract
    pub fn into_cosmos_msg<T: Into<String>>(self, contract_addr: T) -> StdResult<CosmosMsg> {
        let msg = self.into_binary()?;
        let execute = WasmMsg::Execute {
            contract_addr: contract_addr.into(),
            msg,
            funds: vec![],
        };
        Ok(execute.into())
    }
}

#[cw_serde]
enum CompetitionEscrowMsg {
    Distribute(CompetitionEscrowDistributeMsg),
}
