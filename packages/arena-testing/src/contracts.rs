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

pub fn arena_league_module_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            arena_league_module::contract::execute,
            arena_league_module::contract::instantiate,
            arena_league_module::contract::query,
        )
        .with_reply(arena_league_module::contract::reply),
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

pub fn proposal_single_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        dao_proposal_single::contract::execute,
        dao_proposal_single::contract::instantiate,
        dao_proposal_single::contract::query,
    )
    .with_reply(dao_proposal_single::contract::reply)
    .with_migrate(dao_proposal_single::contract::migrate);
    Box::new(contract)
}

pub fn dao_dao_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        dao_dao_core::contract::execute,
        dao_dao_core::contract::instantiate,
        dao_dao_core::contract::query,
    )
    .with_reply(dao_dao_core::contract::reply)
    .with_migrate(dao_dao_core::contract::migrate);
    Box::new(contract)
}

pub fn cw4_group_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw4_group::contract::execute,
        cw4_group::contract::instantiate,
        cw4_group::contract::query,
    );
    Box::new(contract)
}

pub fn dao_voting_cw4_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        dao_voting_cw4::contract::execute,
        dao_voting_cw4::contract::instantiate,
        dao_voting_cw4::contract::query,
    )
    .with_reply(dao_voting_cw4::contract::reply);
    Box::new(contract)
}
