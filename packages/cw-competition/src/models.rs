use cosmwasm_schema::cw_serde;

#[cw_serde]
pub enum CompetitionState {
    Pending,
    Staged,
    Active,
    Inactive,
}

impl Default for CompetitionState {
    fn default() -> Self {
        CompetitionState::Inactive
    }
}
impl CompetitionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            CompetitionState::Pending => "Pending",
            CompetitionState::Staged => "Staged",
            CompetitionState::Active => "Active",
            CompetitionState::Inactive => "Inactive",
        }
    }
}
