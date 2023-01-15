use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_binary, Binary, Coin, CosmosMsg, StdResult, WasmMsg};

use crate::{DisbursementDataResponse, MemberShare};

#[cw_serde]
pub enum CwDisbursementExecuteMsg {
    SetDisbursementData {
        key: String,
        disbursement_data: Vec<MemberShare>,
    },
    //this msg should the movement of native coins
    ReceiveNative {
        key: Option<String>, //this is a key for distributions enabling waterfall payments
    },
}

impl CwDisbursementExecuteMsg {
    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = ExecuteMsgWrapper::CwDisbursementExecute(self);
        to_binary(&msg)
    }

    /// creates a cosmos_msg sending this struct to the named contract
    pub fn into_cosmos_msg<T: Into<String>>(
        self,
        contract_addr: T,
        funds: Vec<Coin>,
    ) -> StdResult<CosmosMsg> {
        let msg = self.into_binary()?;
        let execute = WasmMsg::Execute {
            contract_addr: contract_addr.into(),
            msg,
            funds,
        };
        Ok(execute.into())
    }
}

// This is just a helper to properly serialize the above message
#[cw_serde]
enum ExecuteMsgWrapper {
    CwDisbursementExecute(CwDisbursementExecuteMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum CwDisbursementQueryMsg {
    #[returns(DisbursementDataResponse)]
    DisbursementData { key: Option<String> }, //key for loading disbursement data if implemented with a map
}
