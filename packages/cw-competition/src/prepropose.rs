use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Binary, CosmosMsg, StdResult, Uint128, WasmMsg};

#[cw_serde]
pub enum PreProposeQueryMsg {
    QueryExtension { msg: PreProposeQueryExtensionMsg },
}

#[cw_serde]
pub enum PreProposeQueryExtensionMsg {
    Tax { height: Option<u64> },
}

#[cw_serde]
pub enum PreProposeExecuteMsg {
    Extension { msg: PreProposeExecuteExtensionMsg },
}

#[cw_serde]
pub enum PreProposeExecuteExtensionMsg {
    Jail { id: Uint128 },
}

impl PreProposeExecuteExtensionMsg {
    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = PreProposeExecuteMsg::Extension { msg: self };
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
