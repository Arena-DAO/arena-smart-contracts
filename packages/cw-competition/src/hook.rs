use crate::CompetitionState;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Binary, CosmosMsg, StdResult, WasmMsg};
use cw_disbursement::MemberShare;

#[cw_serde]
pub struct CwCompetitionResultMsg {
    pub distribution: Option<Vec<MemberShare>>,
}

impl CwCompetitionResultMsg {
    pub fn new(distribution: Option<Vec<MemberShare>>) -> Self {
        CwCompetitionResultMsg { distribution }
    }

    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = HookMsg::HandleCompetitionResult(self);
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
pub struct CwCompetitionStateChangedMsg {
    pub old_state: CompetitionState,
    pub new_state: CompetitionState,
}

impl CwCompetitionStateChangedMsg {
    pub fn new(old_state: CompetitionState, new_state: CompetitionState) -> Self {
        CwCompetitionStateChangedMsg {
            old_state,
            new_state,
        }
    }

    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = HookMsg::HandleCompetitionStateChanged(self);
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

// This is just a helper to properly serialize the above message
#[cw_serde]
enum HookMsg {
    HandleCompetitionResult(CwCompetitionResultMsg),
    HandleCompetitionStateChanged(CwCompetitionStateChangedMsg),
}
