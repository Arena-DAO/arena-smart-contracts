use crate::CompetitionState;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_binary, Binary, CosmosMsg, StdResult, WasmMsg};
use cw_controllers::{AdminResponse, HooksResponse};

#[cw_serde]
pub enum CwCompetitionExecuteMsg {
    /// Change the admin
    UpdateAdmin {
        admin: Option<String>,
    },
    /// Add a new hook to be informed of all membership changes. Must be called by Admin
    AddHook {
        addr: String,
    },
    /// Remove a hook. Must be called by Admin
    RemoveHook {
        addr: String,
    },
    SetState {
        state: CompetitionState,
    },
    Abort {},
    IncreaseWin {
        addrs: Vec<String>,
    },
}

impl CwCompetitionExecuteMsg {
    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = ExecuteMsgWrapper::CwCompetitionExecute(self);
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
enum ExecuteMsgWrapper {
    CwCompetitionExecute(CwCompetitionExecuteMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum CwCompetitionQueryMsg {
    #[returns(CompetitionState)]
    State {},
    #[returns(HooksResponse)]
    Hooks {},
    #[returns(AdminResponse)]
    Admin {},
}
