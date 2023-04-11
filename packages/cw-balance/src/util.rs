use cosmwasm_std::Deps;

pub fn is_contract(deps: Deps, addr: String) -> bool {
    match deps.querier.query_wasm_contract_info(addr) {
        Ok(_) => true,
        Err(_) => false,
    }
}
