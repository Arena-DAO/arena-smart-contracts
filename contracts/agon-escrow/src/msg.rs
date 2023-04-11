use std::collections::HashMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary};
use cw20::Cw20ReceiveMsg;
use cw721::Cw721ReceiveMsg;
use cw_balance::{Balance, Distribution, DistributionRaw};
use cw_controllers::AdminResponse;

#[cw_serde]
pub struct InstantiateMsg {
    pub dues: HashMap<String, Balance>,
    pub stakes: HashMap<String, Balance>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Withdraw {
        cw20_msg: Option<Binary>,
        cw721_msg: Option<Binary>,
    },
    SetDistribution {
        distribution: DistributionRaw,
    },
    ReceiveNative {},
    Receive(Cw20ReceiveMsg),
    ReceiveNft(Cw721ReceiveMsg),
    Distribute {
        distribution: DistributionRaw,
        remainder_addr: String,
    },
    Lock {
        value: bool,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AdminResponse)]
    Admin {},
    #[returns(Balance)]
    Balance { addr: String },
    #[returns(Balance)]
    Due { addr: String },
    #[returns(Balance)]
    Stake { addr: String },
    #[returns(Balance)]
    TotalBalance {},
    #[returns(bool)]
    IsLocked {},
    #[returns(DumpStateResponse)]
    DumpState {},
    #[returns(Option<Distribution>)]
    Distribution { addr: String },
}

#[cw_serde]
pub struct DumpStateResponse {
    pub admin: Addr,
    pub is_locked: bool,
    pub total_balance: Balance,
}

#[cw_serde]
pub struct MigrateMsg {}
