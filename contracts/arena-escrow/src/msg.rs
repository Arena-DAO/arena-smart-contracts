use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary};
use cw20::Cw20ReceiveMsg;
use cw721::Cw721ReceiveMsg;
use cw_balance::{BalanceVerified, MemberBalance, MemberShare, MemberShareVerified};
use cw_controllers::AdminResponse;

#[cw_serde]
pub struct InstantiateMsg {
    pub dues: Vec<MemberBalance>,
    pub stakes: Vec<MemberBalance>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Withdraw {
        cw20_msg: Option<Binary>,
        cw721_msg: Option<Binary>,
    },
    SetDistribution {
        distribution: Vec<MemberShare>,
    },
    ReceiveNative {},
    Receive(Cw20ReceiveMsg),
    ReceiveNft(Cw721ReceiveMsg),
    Distribute {
        distribution: Vec<MemberShare>,
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
    #[returns(BalanceVerified)]
    Balance { addr: String },
    #[returns(BalanceVerified)]
    Due { addr: String },
    #[returns(BalanceVerified)]
    Stake { addr: String },
    #[returns(bool)]
    IsFunded { addr: String },
    #[returns(bool)]
    IsFullyFunded {},
    #[returns(BalanceVerified)]
    DistributableBalance {},
    #[returns(BalanceVerified)]
    TotalBalance {},
    #[returns(bool)]
    IsLocked {},
    #[returns(DumpStateResponse)]
    DumpState {},
    #[returns(Option<Vec<MemberShareVerified>>)]
    Distribution { addr: String },
}

#[cw_serde]
pub struct DumpStateResponse {
    pub admin: Addr,
    pub is_locked: bool,
    pub total_balance: BalanceVerified,
}

#[cw_serde]
pub struct MigrateMsg {}
