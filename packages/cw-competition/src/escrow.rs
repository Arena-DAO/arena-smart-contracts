use arena_core_interface::fees::FeeInformation;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Binary, CosmosMsg, StdResult, WasmMsg};
use cw_balance::Distribution;

#[cw_serde]
pub struct CompetitionEscrowDistributeMsg {
    pub distribution: Option<Distribution<String>>,
    /// Layered fees is an ordered list of fees to be applied before the distribution.
    /// The term layered refers to the implementation: Arena Tax -> Host Fee? -> Other Fee?
    /// Each fee is calculated based off the available funds at its layer
    pub layered_fees: Option<Vec<FeeInformation<String>>>,
}

impl CompetitionEscrowDistributeMsg {
    /// serializes the message
    pub fn into_json_binary(self) -> StdResult<Binary> {
        let msg = CompetitionEscrowMsg::Distribute(self);
        to_json_binary(&msg)
    }

    /// creates a cosmos_msg sending this struct to the named contract
    pub fn into_cosmos_msg<T: Into<String>>(self, contract_addr: T) -> StdResult<CosmosMsg> {
        let msg = self.into_json_binary()?;
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
