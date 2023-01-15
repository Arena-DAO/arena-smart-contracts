use crate::models::{
    CompetitionModuleInfo, DumpStateResponse, ModuleInstantiateInfo, Ruleset, WagerDAO,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_disbursement::{MemberBalance, MemberShare};
use cw_utils::Expiration;

#[cw_serde]
pub struct InstantiateMsg {
    /// Instantiate information for the contract's
    /// competition modules.
    pub competition_modules_instantiate_info: Vec<ModuleInstantiateInfo>,
    pub rulesets: Vec<Ruleset>,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateCompetitionModules {
        to_add: Vec<ModuleInstantiateInfo>,
        to_disable: Vec<String>,
    },
    /*CreateWagerPost {
        members: String,
        admin: String,
        start_time: Duration,
        duration: Duration,
        group_min: Option<u64>,
        group_max: Option<u64>,
        cw20: Option<String>,
        denom: Option<String>,
        token_amount: u128,
        token_stake: u128,
        rules: Vec<String>,
    },*/
    JailWager {
        id: Uint128,
    },
    CreateWager {
        wager_dao: WagerDAO,
        expiration: Expiration,
        escrow_code_id: u64,
        wager_amount: Vec<MemberBalance>,
        stake: Vec<MemberBalance>,
        rules: Vec<String>,
        ruleset: Option<Uint128>,
    },
    HandleWager {
        id: Uint128,
        distribution: Option<Vec<MemberShare>>,
    },
    UpdateRulesets {
        to_add: Vec<Ruleset>,
        to_disable: Vec<Uint128>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<CompetitionModuleInfo>)]
    CompetitionModules {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(Vec<Ruleset>)]
    Rulesets {
        start_after: Option<u128>,
        limit: Option<u32>,
    },
    #[returns(Addr)]
    DAO {},
    #[returns(Decimal)]
    Tax { height: Option<u64> },
    #[returns(DumpStateResponse)]
    DumpState {},
}

#[cw_serde]
pub struct MigrateMsg {}
