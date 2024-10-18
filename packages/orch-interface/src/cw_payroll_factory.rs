use cw_orch::interface;
use cw_orch::prelude::*;

use cw_payroll_factory::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

pub const CONTRACT_ID: &str = "cw_payroll_factory";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, id = CONTRACT_ID)]
pub struct DaoPayrollFactory;

impl<Chain> Uploadable for DaoPayrollFactory<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path(CONTRACT_ID)
            .unwrap()
    }
    /// Returns a CosmWasm contract wrapper
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(
            ContractWrapper::new_with_empty(
                cw_payroll_factory::contract::execute,
                cw_payroll_factory::contract::instantiate,
                cw_payroll_factory::contract::query,
            )
            .with_reply(cw_payroll_factory::contract::reply)
            .with_migrate(cw_payroll_factory::contract::migrate),
        )
    }
}
