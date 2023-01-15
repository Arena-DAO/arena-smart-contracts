use std::ops::Deref;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Addr, CosmosMsg, QuerierWrapper, StdResult, WasmMsg};

use crate::{
    CwDisbursementExecuteMsg, CwDisbursementQueryMsg, DisbursementDataResponse, MemberShare,
};

#[cw_serde]
pub struct CwDisbursementContract(pub Addr);

impl Deref for CwDisbursementContract {
    type Target = Addr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl CwDisbursementContract {
    pub fn new(addr: Addr) -> Self {
        CwDisbursementContract(addr)
    }

    fn encode_msg(&self, msg: CwDisbursementExecuteMsg) -> StdResult<CosmosMsg> {
        Ok(WasmMsg::Execute {
            contract_addr: self.to_string(),
            msg: to_binary(&msg)?,
            funds: vec![],
        }
        .into())
    }

    pub fn set_disbursement_data(
        &self,
        key: &String,
        disbursement_data: &Vec<MemberShare>,
    ) -> StdResult<CosmosMsg> {
        self.encode_msg(CwDisbursementExecuteMsg::SetDisbursementData {
            disbursement_data: disbursement_data.clone(),
            key: key.clone(),
        })
    }

    pub fn disburse(&self, key: &Option<String>) -> StdResult<CosmosMsg> {
        self.encode_msg(CwDisbursementExecuteMsg::ReceiveNative { key: key.clone() })
    }

    pub fn is_disbursement_contract(&self, querier: &QuerierWrapper, key: &Option<String>) -> bool {
        let query_response: StdResult<DisbursementDataResponse> = querier.query_wasm_smart(
            self.0.to_string(),
            &CwDisbursementQueryMsg::DisbursementData { key: key.clone() },
        );

        query_response.is_ok()
    }
}
