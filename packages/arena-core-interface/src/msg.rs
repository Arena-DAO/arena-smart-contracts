use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use dao_interface::state::ModuleInstantiateInfo;
use dao_pre_propose_base::{
    msg::{ExecuteMsg as ExecuteBase, InstantiateMsg as InstantiateBase, QueryMsg as QueryBase},
    state::PreProposeContract,
};
use dao_voting::multiple_choice::MultipleChoiceOptions;

#[cw_serde]
pub struct InstantiateExt {
    pub competition_modules_instantiate_info: Vec<ModuleInstantiateInfo>,
    pub rulesets: Vec<NewRuleset>,
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
        proposal_details: ProposalDetails,
    },
    UpdateTax {
        tax: Decimal,
    },
    UpdateRulesets {
        to_add: Vec<NewRuleset>,
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
    #[returns(Option<Ruleset>)]
    Ruleset { id: Uint128 },
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

pub type InstantiateMsg = InstantiateBase<InstantiateExt>;
pub type ExecuteMsg = ExecuteBase<ProposeMessage, ExecuteExt>;
pub type QueryMsg = QueryBase<QueryExt>;
pub type PrePropose = PreProposeContract<InstantiateExt, ExecuteExt, QueryExt, ProposeMessage>;

#[cw_serde]
pub struct DumpStateResponse {
    pub tax: Decimal,
    pub competition_modules: Vec<CompetitionModuleResponse>,
    pub rulesets: Vec<Ruleset>,
}

#[cw_serde]
pub struct CompetitionModuleResponse {
    pub key: String,
    pub addr: Addr,
    pub is_enabled: bool,
    pub competition_count: Uint128,
}

#[cw_serde]
pub struct NewRuleset {
    pub rules: Vec<String>,
    pub description: String,
}

#[cw_serde]
pub struct Ruleset {
    pub id: Uint128,
    pub rules: Vec<String>,
    pub description: String,
    pub is_enabled: bool,
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

#[cw_serde]
pub struct ProposalDetails {
    pub title: String,
    pub description: String,
}
