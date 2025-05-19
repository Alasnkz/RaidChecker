use std::fs::{self, File};
use std::path::Path;
use std::io::{self, Write};

use crate::checker::check_player::PlayerData;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct LastRaid {
    pub raid_url: String,   
    pub raid_name: String,
    pub players: Vec<PlayerData>
}

impl Default for LastRaid    {
    fn default() -> Self {
        Self {
            raid_url: String::default(),
            raid_name: String::default(),
            players: Vec::new()
        }
    }
}

impl LastRaid {
    pub fn read_or_create<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        // Try to read the file
        if path.as_ref().exists() {
            let content = fs::read_to_string(&path).unwrap();
            match serde_json::from_str(&content) {
                Ok(config) => Ok(config),
                Err(err) => {
                    eprintln!("Error parsing config: {}. Creating new default config.", err);
                    Ok(LastRaid::default())
                }
            }
        } else {
            Ok(LastRaid::default())
        }
    }

    pub fn save(&self) {
        let json = serde_json::to_string_pretty(self).unwrap();
        let mut file = File::create("last_raid.json").unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }

    pub fn save_mut(&mut self) {
        let json = serde_json::to_string_pretty(self).unwrap();
        let mut file = File::create("last_raid.json").unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }
}