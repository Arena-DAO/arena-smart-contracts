use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_utils::Expiration;

#[cw_serde]
pub enum WagerDAO {
    New {
        dao_code_id: u64,
        group_code_id: u64,
        proposal_code_id: u64,
        members: Vec<String>,
    },
    Existing {
        addr: String,
    },
}

#[cw_serde]
pub enum WagerAmount {
    New {
        dao_code_id: u64,
        voting_code_id: u64,
        group_code_id: u64,
        members: Vec<String>,
    },
    Existing {
        addr: String,
    },
}

#[cw_serde]
pub enum WagerStatus {
    Pending,
    Active,
    Inactive,
    Jailed,
}

#[cw_serde]
pub struct Wager {
    pub dao: Addr,
    pub expiration: Expiration,
    pub escrow: Addr,
    pub rules: Vec<String>,
    pub ruleset: Option<Uint128>,
    pub evidence: Option<String>,
    pub status: WagerStatus,
}
