extern crate reqwest;
extern crate select;
#[macro_use] extern crate failure;
#[macro_use] extern crate derive_builder;
#[macro_use] extern crate log;

use failure::Error;
use std::io::Read;
use select::document::Document;
use select::predicate::{Class, Predicate, Name};
use chrono::Duration;

#[derive(Builder, Debug)]
pub struct ScoreboardEntry {
    #[builder(default)]
    position: Option<u32>,
    number: u32,
    team: String,
    entrant: String,
    #[builder(default)]
    lap_last: Option<Duration>,
    #[builder(default)]
    lap_best: Option<Duration>,
    #[builder(default)]
    speed: Option<f32>,
    #[builder(default)]
    laps: Option<u32>,
    #[builder(default)]
    distance: Option<f32>
}
fn parse_lap_time(time: &str) -> Result<Duration, Error> {
    if let Some(ap) = time.find('\'') {
        if let Some(colon) = time.find(':') {
            // hours, minutes, and seconds, like 1:2'42.0
            if time.len() == colon + 1 || time.len() == ap + 1 {
                return Err(format_err!("time ends in colon or apostrophe"))?;
            }
            let hours: u32 = time[0..colon].parse()?;
            let minutes: u32 = time[colon+1..ap].parse()?;
            let secs: f32 = time[ap+1..].parse()?;
            Ok(
                Duration::hours(hours as _) +
                Duration::minutes(minutes as _) + 
                Duration::milliseconds((secs * 1000.0) as _)
            )
        }
        else {
            // minutes and seconds, like 1'42.0
            if time.len() == ap + 1 {
                return Err(format_err!("time ends in apostrophe"))?;
            }
            let minutes: u32 = time[0..ap].parse()?;
            let secs: f32 = time[ap+1..].parse()?;
            Ok(
                Duration::minutes(minutes as _) + 
                Duration::milliseconds((secs * 1000.0) as _)
            )
        }
    }
    else {
        // time in seconds, like 42.0
        let secs: f32 = time.parse()?;
        Ok(Duration::milliseconds((secs * 1000.0) as _))
    }
}
fn fetch_scoreboard(url: &str) -> Result<Vec<ScoreboardEntry>, Error> {
    let mut resp = reqwest::get(url)?;
    info!("Fetching scoreboard at {}", url);
    if !resp.status().is_success() {
        return Err(format_err!("Request returned error code: {}", resp.status()));
    }
    info!("Response: {}", resp.status());
    let mut content = Vec::new();
    resp.read_to_end(&mut content)?;
    let content = String::from_utf8_lossy(&content);
    let doc = Document::from(&content as &str);
    let table = doc.find(Name("table").and(Class("NBT")))
        .into_iter()
        .nth(0)
        .ok_or(format_err!("No table found"))?;
    let mut rows = table.find(Name("tr"));
    let first = rows.next().ok_or(format_err!("No header row"))?;
    let mut names = vec![];
    let mut entries = vec![];
    for cell in first.find(Name("td")) {
        names.push(cell.inner_html().to_lowercase());
    }
    for row in rows {
        let mut entry = ScoreboardEntryBuilder::default();
        for (i, cell) in row.find(Name("td")).enumerate() {
            if let Some(name) = names.get(i) {
                match name as &str {
                    x @ "p" | x @ "#lps" => {
                        if let Ok(pos) = cell.inner_html().parse() {
                            match x {
                                "p" => {
                                    entry.position(Some(pos));
                                },
                                "#lps" => {
                                    entry.laps(Some(pos));
                                },
                                _ => unreachable!()
                            }
                        }
                    },
                    x @ "spd" | x @ "dist" => {
                        if let Ok(pos) = cell.inner_html().parse() {
                            match x {
                                "spd" => {
                                    entry.speed(Some(pos));
                                },
                                "dist" => {
                                    entry.distance(Some(pos));
                                },
                                _ => unreachable!()
                            }
                        }
                    },
                    "#" => {
                        if let Ok(num) = cell.inner_html().parse() {
                            entry.number(num);
                        }
                    },
                    "team" => {
                        entry.team(cell.inner_html());
                    },
                    "entrant" => {
                        entry.entrant(cell.inner_html());
                    },
                    x @ "last" | x @ "best" => {
                        match parse_lap_time(&cell.inner_html()) {
                            Ok(lt) => {
                                if x == "last" {
                                    entry.lap_last(Some(lt));
                                }
                                else {
                                    entry.lap_best(Some(lt));
                                }
                            },
                            Err(e) => {
                                warn!("Invalid lap time \"{}\": {}", cell.inner_html(), e);
                            }
                        }
                    },
                    x if x.trim() == "" => {},
                    x => {
                        warn!("Unknown table column {}", x);
                    },
                }
            }
        }
        match entry.build() {
            Ok(e) => entries.push(e),
            Err(e) => {
                warn!("Failed to build entry: {}", e);
            },
        }
    }
    Ok(entries)
}
fn main() {
    env_logger::init();
    println!("{:#?}", fetch_scoreboard("http://www.bbk-online.net/gpt/race489.php"));
    println!("{:#?}", fetch_scoreboard("http://www.bbk-online.net/gpt/race500.php"));
}
