use cosmwasm_std::WasmMsg;

use crate::models::ModuleInstantiateInfo;

impl ModuleInstantiateInfo {
    pub fn into_wasm_msg(self, admin: Option<String>) -> WasmMsg {
        WasmMsg::Instantiate {
            admin,
            code_id: self.code_id,
            msg: self.msg,
            funds: vec![],
            label: self.label,
        }
    }
}
