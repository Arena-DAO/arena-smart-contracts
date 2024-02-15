use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, Binary, CosmosMsg, Decimal, Deps, StdResult, WasmMsg};
use cw_address_like::AddressLike;
use cw_balance::MemberPercentage;

#[cw_serde]
pub struct TaxInformation<T: AddressLike> {
    pub tax: Decimal,
    pub receiver: T,
    pub cw20_msg: Option<Binary>,
    pub cw721_msg: Option<Binary>,
}

impl TaxInformation<String> {
    pub fn into_checked(&self, deps: Deps) -> StdResult<TaxInformation<Addr>> {
        Ok(TaxInformation {
            receiver: deps.api.addr_validate(&self.receiver)?,
            tax: self.tax,
            cw20_msg: self.cw20_msg.clone(),
            cw721_msg: self.cw721_msg.clone(),
        })
    }
}

#[cw_serde]
pub struct CompetitionEscrowDistributeMsg {
    pub distribution: Vec<MemberPercentage<String>>,
    pub tax_info: Option<TaxInformation<String>>,
    pub remainder_addr: String,
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
