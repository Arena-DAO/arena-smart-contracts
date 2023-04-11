use std::time::Duration;

use cosmwasm_schema::cw_serde;
use dao_interface::ModuleInstantiateInfo;

#[cw_serde]
pub struct InstantiateBase<InstantiateExt> {
    pub name: String,
    pub description: String,
    pub rules: Vec<String>,
    pub active_duration: Duration,
    pub escrow: ModuleInstantiateInfo,
    pub extension: InstantiateExt,
}
