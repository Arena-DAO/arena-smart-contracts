use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn agon_core_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        agon_core::contract::execute,
        agon_core::contract::instantiate,
        agon_core::contract::query,
    ))
}

pub fn agon_escrow_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        agon_escrow::contract::execute,
        agon_escrow::contract::instantiate,
        agon_escrow::contract::query,
    ))
}

pub fn dao_proposal_multiple_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        dao_proposal_multiple::contract::execute,
        dao_proposal_multiple::contract::instantiate,
        dao_proposal_multiple::contract::query,
    ))
}
