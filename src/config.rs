use std::collections::HashMap;

#[derive(Deserialize)]
pub struct RaceConfig {
    pub url: String,
    pub race_id: i32,
}
#[derive(Deserialize)]
pub struct Config {
    pub database_url: String,
    pub team_mappings: HashMap<String, i32>,
    pub races: Vec<RaceConfig>
}
