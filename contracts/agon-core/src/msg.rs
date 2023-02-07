use crate::models::{CompetitionModule, DumpStateResponse, Ruleset, WagerDAO};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Empty, Uint128};
use cw_disbursement::{MemberBalance, MemberShare};
use cw_utils::Expiration;
use dao_interface::ModuleInstantiateInfo;
use dao_pre_propose_base::{
    msg::{ExecuteMsg as ExecuteBase, InstantiateMsg as InstantiateBase, QueryMsg as QueryBase},
    state::PreProposeContract,
};

#[cw_serde]
pub struct InstantiateExt {
    pub competition_modules_instantiate_info: Vec<ModuleInstantiateInfo>,
    pub rulesets: Vec<Ruleset>,
    pub tax: Decimal,
}

#[cw_serde]
pub enum ExecuteExt {
    UpdateCompetitionModules {
        to_add: Vec<ModuleInstantiateInfo>,
        to_remove: Vec<String>,
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
    UpdateTax {
        tax: Decimal,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryExt {
    #[returns(Vec<CompetitionModule>)]
    CompetitionModules {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(Vec<Ruleset>)]
    Rulesets {
        skip: Option<u32>,
        limit: Option<u32>,
        description: Option<String>,
    },
    #[returns(Decimal)]
    Tax { height: Option<u64> },
    #[returns(DumpStateResponse)]
    DumpState {},
}

pub type InstantiateMsg = InstantiateBase<InstantiateExt>;
pub type ExecuteMsg = ExecuteBase<Empty, ExecuteExt>;
pub type QueryMsg = QueryBase<QueryExt>;
pub type PrePropose = PreProposeContract<InstantiateExt, ExecuteExt, QueryExt, Empty>;
