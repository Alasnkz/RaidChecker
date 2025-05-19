use std::fs::{self, File};
use std::path::Path;
use std::io::{self, Write};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ExpansionEnchants {
    pub slot: String,
    pub sub_slots: Vec<String>, // things like TWOHWEAPON... is a weapon.
    pub enchant_ids: Vec<i32>,
    pub lesser_enchant_ids: Option<Vec<i32>>, // Some enchants are lesser enchants, it would be useful to warn about them. Currently, these are only used for corruptions (TWW S2).
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RaidDifficulty {
    pub difficulty_name: String,
    pub id: i32
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RaidReputation {
    pub raid_rep_slug: String,
    pub raid_buff_renowns: Vec<i32>,
    pub renown_start: i64, // Timestamp
    pub max_renown_value_weekly: i32,
    pub renown_level_value: i32,
    pub buff_size: i32
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ExpansionRaid {
    pub identifier: String,
    pub difficulty: Vec<RaidDifficulty>,
    pub id: i32,
    pub boss_names: Vec<String>,
    pub aotc_achievement_id: i32,
    pub reputation: Option<RaidReputation>
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Expansions {
    pub name: String,
    pub identifier: String, // <-- TWW, MN, TLT
    pub reputation_slug: String,
    pub gear_embelishment_bonus_id: i32,
    pub gear_enchants: Vec<ExpansionEnchants>,
    pub raids: Vec<ExpansionRaid>
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ExpansionsConfig {
    pub rhcu_version: i32,
    pub latest_expansion_identifier: String,
    pub expansions: Vec<Expansions>,
    pub latest_expansion: Option<Expansions>
}

impl Default for ExpansionsConfig {
    fn default() -> Self {
        Self {
            rhcu_version: 1,
            latest_expansion_identifier: "TWW".to_owned(),
            expansions: Vec::new(),
            latest_expansion: None
        }
    }
}

impl ExpansionsConfig {
    fn create_default<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let settings = ExpansionsConfig::default();
        let json = serde_json::to_string_pretty(&settings).unwrap();
        let mut file = File::create(path).unwrap();
        file.write_all(json.as_bytes()).unwrap();
        Ok(settings)
    }

    pub fn read_or_create<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        // Try to read the file
        if path.as_ref().exists() {
            let content = fs::read_to_string(&path).unwrap();
            match serde_json::from_str(&content) {
                Ok(config) => Ok(config),
                Err(err) => {
                    eprintln!("Error parsing config: {}. Creating new default config.", err);
                    Self::create_default(path)
                }
            }
        } else {
            Self::create_default(path)
        }
    }
}