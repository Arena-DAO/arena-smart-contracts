use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn arena_dao_core_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            arena_core::contract::execute,
            arena_core::contract::instantiate,
            arena_core::contract::query,
        )
        .with_reply(arena_core::contract::reply),
    )
}

pub fn arena_dao_escrow_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        arena_escrow::contract::execute,
        arena_escrow::contract::instantiate,
        arena_escrow::contract::query,
    ))
}

pub fn arena_wager_module_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            arena_wager_module::contract::execute,
            arena_wager_module::contract::instantiate,
            arena_wager_module::contract::query,
        )
        .with_reply(arena_wager_module::contract::reply),
    )
}

pub fn dao_proposal_multiple_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            dao_proposal_multiple::contract::execute,
            dao_proposal_multiple::contract::instantiate,
            dao_proposal_multiple::contract::query,
        )
        .with_reply(dao_proposal_multiple::contract::reply),
    )
}

pub fn cw20_base_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    ))
}

pub fn cw721_base_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        cw721_base::entry::execute,
        cw721_base::entry::instantiate,
        cw721_base::entry::query,
    ))
}
