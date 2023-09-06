#[allow(unused_imports)]
use crate::state::{CompetitionModule, Ruleset};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};
use dao_interface::state::ModuleInstantiateInfo;
use dao_pre_propose_base::{
    msg::{ExecuteMsg as ExecuteBase, InstantiateMsg as InstantiateBase, QueryMsg as QueryBase},
    state::PreProposeContract,
};
use dao_voting::multiple_choice::MultipleChoiceOptions;

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
        to_disable: Vec<String>,
    },
    Jail {
        id: Uint128,
        title: String,
        description: String,
    },
    UpdateTax {
        tax: Decimal,
    },
    UpdateRulesets {
        to_add: Vec<Ruleset>,
        to_disable: Vec<Uint128>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryExt {
    #[returns(Vec<(String, CompetitionModule)>)]
    CompetitionModules {
        start_after: Option<String>,
        limit: Option<u32>,
        include_disabled: Option<bool>,
    },
    #[returns(Vec<(u128, Ruleset)>)]
    Rulesets {
        start_after: Option<u128>,
        limit: Option<u32>,
        include_disabled: Option<bool>,
    },
    #[returns(Decimal)]
    Tax { height: Option<u64> },
    #[returns(Option<String>)]
    CompetitionModule { key: String },
}

#[cw_serde]
pub enum MigrateMsg {
    FromV1 {},
}

#[cw_serde]
pub enum ProposeMessage {
    Propose {
        title: String,
        description: String,
        choices: MultipleChoiceOptions,
        proposer: Option<String>,
    },
}

pub type InstantiateMsg = InstantiateBase<InstantiateExt>;
pub type ExecuteMsg = ExecuteBase<ProposeMessage, ExecuteExt>;
pub type QueryMsg = QueryBase<QueryExt>;
pub type PrePropose = PreProposeContract<InstantiateExt, ExecuteExt, QueryExt, ProposeMessage>;
