extern crate reqwest;
extern crate select;
#[macro_use] extern crate failure;
#[macro_use] extern crate derive_builder;
#[macro_use] extern crate log;
extern crate postgres;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate toml;

pub mod scraper;
pub mod config;

use self::config::Config;
use postgres::{Connection, TlsMode};

fn main() {
    env_logger::init();
    info!("[+] F24 BBK Lap Time Scraper");
    info!("[+] Reading configuration");
    let config = std::env::args().nth(1).expect("Config file should be first argument");
    let cfg_text = std::fs::read_to_string(&config).unwrap();
    let cfg: Config = toml::from_str(&cfg_text).unwrap();
    info!("[+] Connecting to PostgreSQL");
    let conn = Connection::connect(&cfg.database_url as &str, TlsMode::None)
        .unwrap();
    for race in cfg.races {
        info!("[+] Fetching data from: {}", race.url);
        match scraper::fetch_scoreboard(&race.url) {
            Ok(d) => {
                let mut times = 0;
                for ent in d {
                    let team_id = cfg.team_mappings.get(&ent.team)
                        .map(|x| *x as i32);
                    if let Some(ln) = ent.laps {
                        let lap: i32 = ln as _;
                        if let Some(ll) = ent.lap_last {
                            let millis: i32 = ll.num_milliseconds() as _;
                            conn.execute("INSERT INTO laptimes
                                          (race_id, car_id, team_num, team_name, entrant_name, lap_no, lap_time_ms)
                                          VALUES ($1, $2, $3, $4, $5, $6, $7)
                                          ON CONFLICT DO NOTHING",
                                          &[&race.race_id, &team_id, &(ent.number as i32), &ent.team, &ent.entrant, &lap, &millis])
                                .unwrap();
                            times += 1;
                        }
                    }
                }
                info!("[+] Posted {} lap times", times);
            }
            Err(e) => {
                warn!("[!] Scraping failed: {}", e);
            }
        }
    }
    info!("[+] Scraping done!");
}
