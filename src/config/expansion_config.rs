use std::fs::{self, File};
use std::path::Path;
use std::io::{self, Write};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub struct ExpansionEnchants {
    pub slot: String,
    #[serde(default="default_vec")]
    pub sub_slots: Vec<String>, // things like TWOHWEAPON... is a weapon.
    #[serde(default="default_vec")]
    pub enchant_ids: Vec<i32>,
    pub lesser_enchant_ids: Option<Vec<i32>>, // Some enchants are lesser enchants, it would be useful to warn about them. Currently, these are only used for corruptions (TWW S2).
    pub special_item_id: Option<Vec<i32>>,
    #[serde(default="default_false")]
    pub has_socket: bool,
}

fn default_false() -> bool {
    false
}

fn default_vec<T: Default>() -> Vec<T> {
    Vec::new()
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
    #[serde(default="default_i32")]
    pub aotc_achievement_id: i32,
    pub reputation: Option<RaidReputation>,
}

fn default_i32() -> i32 {
    -1
}

impl Default for ExpansionRaid {
    fn default() -> Self {
        Self {
            identifier: "Unknown".to_owned(),
            difficulty: Vec::new(),
            id: -1,
            boss_names: Vec::new(),
            aotc_achievement_id: -1,
            reputation: None,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct ExpansionSeasons {
    pub seasonal_identifier: String,
    #[serde(default="default_i64")]
    pub season_start: i64,
    pub raids: Vec<ExpansionRaid>,
    pub seasonal_gear: Option<Vec<ExpansionEnchants>> // Contains data for things such as D.I.S.C. belt, or things like seasonal enchants (horrific visions)
}

fn default_i64() -> i64 {
    0
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Expansions {
    pub name: String,
    pub identifier: String, // <-- TWW, MN, TLT
    pub reputation_slug: String,
    pub gear_embelishment_bonus_id: i32,
    pub gear_enchants: Vec<ExpansionEnchants>,
    pub seasons: Vec<ExpansionSeasons>,
    
    #[serde(skip)]
    pub latest_season: Option<ExpansionSeasons>,
}

impl Expansions {
    pub fn find_raid_by_id(&self, raid_id: i32) -> Option<&ExpansionRaid> {
        self.seasons.iter()
            .find_map(|season| season.raids.iter().find(|raid| raid.id == raid_id))
    }
}

impl Default for Expansions {
    fn default() -> Self {
        Self {
            name: "Unknown".to_owned(),
            identifier: "Unknown".to_owned(),
            reputation_slug: "Unknown".to_owned(),
            gear_embelishment_bonus_id: -1,
            gear_enchants: Vec::new(),
            seasons: Vec::new(),
            latest_season: None,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ExpansionsConfig {
    pub rhcu_version: String,
    pub modified: u64,
    pub latest_expansion_identifier: String,
    #[serde(default="default_vec")]
    pub agnostic_gear_enchants: Vec<ExpansionEnchants>,
    pub expansions: Vec<Expansions>,
    pub latest_expansion: Option<Expansions>
}

impl Default for ExpansionsConfig {
    fn default() -> Self {
        Self {
            rhcu_version: "0.0.0".to_owned(),
            modified: 0,
            latest_expansion_identifier: "TWW".to_owned(),
            agnostic_gear_enchants: Vec::new(),
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