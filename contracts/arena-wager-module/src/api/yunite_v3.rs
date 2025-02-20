use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct Team {
    pub team_id: String,
    pub users: Vec<User>,
    pub placement: i32,
    pub kills: i32,
    pub counted_kills: i32,
    pub games: i32,
    pub counted_games: i32,
    pub wins: i32,
    pub counted_wins: i32,
    pub placement_score: i32,
    pub elimination_score: i32,
    pub score: i32,
    pub kpm: f64,
    pub average_placement: f64,
    pub sum_seconds_survived: f64,
    pub average_seconds_survived: f64,
    pub game_list: Vec<Game>,
    pub corrections: Vec<Correction>,
}

#[cw_serde]
pub struct User {
    pub index: i32,
    pub discord_id: Option<String>,
    pub epic_id: String,
}

#[cw_serde]
pub struct Game {
    pub placement: i32,
    pub kills: i32,
    pub survival_time: f64,
    pub score: i32,
    pub session_id: String,
    pub counts: bool,
}

#[cw_serde]
pub struct Correction {
    pub amount: i32,
    pub reason: String,
    pub executor_id: String,
    pub timestamp: String,
}
