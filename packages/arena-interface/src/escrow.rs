use crate::fees::FeeInformation;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;
use cw20::Cw20ReceiveMsg;
use cw721::Cw721ReceiveMsg;
#[allow(unused_imports)]
use cw_balance::{
    BalanceVerified, Distribution, MemberBalanceChecked, MemberBalanceUnchecked, MemberPercentage,
};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_ownable_execute]
#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
pub enum ExecuteMsg {
    Withdraw {
        cw20_msg: Option<Binary>,
        cw721_msg: Option<Binary>,
    },
    SetDistribution {
        distribution: Option<Distribution<String>>,
    },
    Activate {},
    #[cw_orch(payable)]
    ReceiveNative {},
    Receive(Cw20ReceiveMsg),
    ReceiveNft(Cw721ReceiveMsg),
    Distribute {
        distribution: Option<Distribution<String>>,
        /// Layered fees is an ordered list of fees to be applied before the distribution.
        /// The term layered refers to the implementation: Arena Tax -> Host Fee? -> Other Fee?
        /// Each fee is calculated based off the available funds at its layer
        layered_fees: Option<Vec<FeeInformation<String>>>,
    },
    Lock {
        value: bool,
    },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
pub enum QueryMsg {
    #[returns(Vec<MemberBalanceChecked>)]
    Balances {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(Option<BalanceVerified>)]
    Balance { addr: String },
    #[returns(Option<BalanceVerified>)]
    Due { addr: String },
    #[returns(Vec<MemberBalanceChecked>)]
    Dues {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(Vec<MemberBalanceChecked>)]
    InitialDues {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(bool)]
    IsFunded { addr: String },
    #[returns(bool)]
    IsFullyFunded {},
    #[returns(Option<BalanceVerified>)]
    TotalBalance {},
    #[returns(bool)]
    IsLocked {},
    #[returns(Option<Distribution<String>>)]
    Distribution { addr: String },
    #[returns(DumpStateResponse)]
    DumpState { addr: Option<String> },
    #[returns(bool)]
    ShouldActivateOnFunded {},
}

#[cw_serde]
pub struct DumpStateResponse {
    pub is_locked: bool,
    pub total_balance: Option<BalanceVerified>,
    pub balance: Option<BalanceVerified>,
    pub due: Option<BalanceVerified>,
}
