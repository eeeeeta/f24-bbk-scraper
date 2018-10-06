use failure::Error;
use std::io::Read;
use select::document::Document;
use select::predicate::{Class, Predicate, Name};
use chrono::Duration;

#[derive(Builder, Debug)]
pub struct ScoreboardEntry {
    #[builder(default)]
    pub position: Option<u32>,
    pub number: u32,
    pub team: String,
    pub entrant: String,
    #[builder(default)]
    pub lap_last: Option<Duration>,
    #[builder(default)]
    pub lap_best: Option<Duration>,
    #[builder(default)]
    pub speed: Option<f32>,
    #[builder(default)]
    pub laps: Option<u32>,
    #[builder(default)]
    pub distance: Option<f32>
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
pub fn fetch_scoreboard(url: &str) -> Result<Vec<ScoreboardEntry>, Error> {
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
                    "result" => {
                        let text = cell.inner_html();
                        if let Some(li) = text.find('L') {
                            if let Ok(no) = text[0..li].parse() {
                                entry.laps(Some(no));
                            }
                            else {
                                warn!("couldn't parse result {}", &text[0..li]);
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
                    x @ "last" | x @ "l-lap" | x @ "best" => {
                        let time = cell.inner_html();
                        match parse_lap_time(&time) {
                            Ok(lt) => {
                                if x == "last" || x == "l-lap" {
                                    entry.lap_last(Some(lt));
                                }
                                else {
                                    entry.lap_best(Some(lt));
                                }
                            },
                            Err(e) => {
                                if time.trim() != "" {
                                    warn!("Invalid lap time \"{}\": {}", time, e);
                                }
                            }
                        }
                    },
                    "gap" => {},
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

