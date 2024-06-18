#[allow(unused_imports)]
use crate::query::DumpStateResponse;
use cosmwasm_schema::cw_serde;
#[allow(unused_imports)]
use cw_balance::{
    BalanceVerified, Distribution, MemberBalanceChecked, MemberBalanceUnchecked, MemberPercentage,
};

#[cw_serde]
pub struct InstantiateMsg {
    pub dues: Vec<MemberBalanceUnchecked>,
    /// Determines if the competition is automatically activated if all dues are paid
    /// Defaults to true
    pub should_activate_on_funded: Option<bool>,
}

#[cw_serde]
pub enum MigrateMsg {
    FromCompatible {},
}
