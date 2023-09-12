#[allow(unused_imports)]
use crate::query::{CompetitionModuleResponse, DumpStateResponse};
#[allow(unused_imports)]
use crate::state::Ruleset;
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
    #[returns(Vec<CompetitionModuleResponse>)]
    CompetitionModules {
        start_after: Option<String>,
        limit: Option<u32>,
        include_disabled: Option<bool>,
    },
    #[returns(Vec<Ruleset>)]
    Rulesets {
        start_after: Option<Uint128>,
        limit: Option<u32>,
        include_disabled: Option<bool>,
    },
    #[returns(Decimal)]
    Tax { height: Option<u64> },
    #[returns(Option<CompetitionModuleResponse>)]
    CompetitionModule { key: String },
    #[returns(DumpStateResponse)]
    DumpState {},
}

#[cw_serde]
pub enum MigrateMsg {
    FromCompatible {},
}

/// This is used to completely generate schema types
/// QueryExt response types are hidden by the QueryBase mapping to Binary output
#[cw_serde]
pub struct SudoMsg {
    pub dump_state_response: DumpStateResponse,
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
