use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::{Result};
use regex::Regex;
use reqwest::blocking::Client;
use serde::Deserialize;
use tracing::{error, info, warn};

use crate::config::{self, expansion_config::{Expansion, ExpansionsConfig, ItemData}, settings::{Settings, SlotSetting}};

#[allow(dead_code)]
pub struct ArmoryChecker {}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
#[allow(dead_code)]
pub struct GearEnchantment {
    pub enchantment_id: i32
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
#[allow(dead_code)]
pub struct GearInventoryType {
    #[serde(alias = "name")]
    _name: String,
    #[serde(alias = "type")]
    pub gear_type: String
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct GearItem {
    #[serde(alias = "id")]
    pub id: u64,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct GearSockets {
    #[serde(alias = "item")]
    pub item: Option<GearItem>
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
#[allow(dead_code)]
pub struct CharacterGear {
    pub bonus_list: Option<Vec<i32>>,
    pub enchantments: Option<Vec<GearEnchantment>>,
    pub id: i32,
    pub inventory_type: GearInventoryType,
    #[serde(alias = "sockets")]
    pub sockets: Option<Vec<GearSockets>>
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct ArmoryTimestamp {
    #[serde(alias = "epoch")]
    pub epoch: i64,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
#[allow(dead_code)]
pub struct ArmoryCharacter {
    #[serde(alias = "averageItemLevel")]
    pub average_item_level: i32,
    pub gear: HashMap<String, CharacterGear>,
    #[serde(alias = "lastUpdatedTimestamp")]
    pub last_updated_timestamp: ArmoryTimestamp,
    pub level: u8,
}

impl Default for ArmoryCharacter {
    fn default() -> Self {
        ArmoryCharacter {
            average_item_level: 0,
            gear: HashMap::new(),
            last_updated_timestamp: ArmoryTimestamp { epoch: 0 },
            level: 0,
        }
    }
}

#[derive(serde::Deserialize,Clone,Debug,PartialEq, Eq, Hash)]
pub struct ArmoryRaidBosses {
    #[serde(alias = "killCount")]
    pub kill_count: i32,
    #[serde(alias = "lastTimestamp")]
    pub last_timestamp: Option<u64>,
    pub name: String
}

#[derive(serde::Deserialize,Clone,Debug,PartialEq, Eq, Hash)]
pub struct ArmoryRaidDifficulty {
    pub name: String,
    pub count: i32,
    pub total: i32,
    pub bosses: Vec<ArmoryRaidBosses>
}

#[derive(serde::Deserialize,Clone)]
#[allow(dead_code)]
pub struct ArmoryRaids {
    pub difficulties: Vec<ArmoryRaidDifficulty>,
    pub name: String
}

#[derive(serde::Deserialize, Clone)]
#[allow(dead_code)]
pub struct ArmorySummary {
    pub raids: Vec<ArmoryRaids>
}

#[derive(serde::Deserialize, Clone)]
#[allow(dead_code)]
pub struct ArmoryCharacterResponse {
    #[serde(skip_deserializing, alias = "lqip")]
    _lqip: Option<String>,
    pub character: ArmoryCharacter,
    pub summary: ArmorySummary,
}

#[derive(serde::Deserialize, Clone)]
#[allow(dead_code)]
pub struct Achievements {
    #[serde(alias = "accountWide")]
    pub account_wide: bool,
    pub description: String,
    pub id: i32,
    pub name: String
}

#[derive(serde::Deserialize, Clone)]
#[allow(dead_code)]
pub struct AchievementSubCategory {
    pub achievements: Vec<Achievements>,
    pub id: String,
    pub name: String
}

#[derive(serde::Deserialize, Clone)]
#[allow(dead_code)]
pub struct AchievementCategory {
    //#[serde(skip_deserializing)]
    //achievementsList: Option<Vec<String>>,
    pub subcategories: HashMap<String, AchievementSubCategory>
}

#[derive(serde::Deserialize, Clone)]
#[allow(dead_code)]
pub struct ArmoryCharacterAchievementResponse {
    #[serde(alias = "achievementCategory")]
    pub achievement_category: AchievementCategory,
}

#[derive(Debug, serde::Deserialize, Clone)]
#[allow(dead_code)]
pub struct ReputationsResponse {
    pub region: String,
    pub reputations: Vec<ReputationCategory>,
}

#[derive(Debug, serde::Deserialize, Clone)]
#[allow(dead_code)]
pub struct ReputationCategory {
    pub id: String,
    pub name: String,
    pub max: bool,
    #[serde(default, alias = "maxValue")]
    pub max_value: Option<u32>,
    #[serde(default)]
    pub standing: Option<String>,
    #[serde(default)]
    pub value: Option<u32>,
    #[serde(default, alias = "standingType")]
    pub standing_type: Option<StandingType>,
    #[serde(default)]
    pub reputations: Vec<ReputationCategory>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct StandingType {
    #[serde(alias = "enum")]
    pub enum_name: String,
    pub id: Option<u32>,
    pub name: String,
    pub slug: String,
}

#[derive(serde::Deserialize, Clone)]
#[allow(dead_code)]
pub struct ArmoryCharacterReputationResponse {
    pub reputations: ReputationsResponse
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, PartialEq)]
pub enum RaidProgressStatus {
    None,
    Account,
    Character,
    CuttingEdge(bool, bool, bool), // Account, Character, Charcter Heroic Kill
    EndBossKilled(bool, bool, bool),
    Skipped,
    Error
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PlayerRaidBossDifficultyData {
    pub difficulty_id: usize,
    pub difficulty_name: String,
    pub boss_kill_time: Option<u64>,
    pub killed_before: bool
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PlayerRaidBossData {
    pub boss_id: usize,
    pub boss_name: String,
    pub difficulties: BTreeMap<usize, PlayerRaidBossDifficultyData>
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PlayerRaidData {
    pub raid_name: String,
    pub bosses: BTreeMap<usize, PlayerRaidBossData>
}

impl ArmoryChecker {
    pub fn check_armory(name_url: &str) -> Option<ArmoryCharacterResponse> {
        let client = Client::new();
        let response = client
            .get(name_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36")
            .send();

        if response.is_err() {
            error!("Error getting armory response: {:?}", response.err());
            return None;
        }

        let text = response.unwrap().text();
        if text.is_err() {
            error!("Error getting armory response (text): {:?}", text.err());
            return None;
        }
        let re = Regex::new(r#"var\s+characterProfileInitialState\s*=\s*(\{.*?\});"#).unwrap();
        if let Some(captures) = re.captures(&text.unwrap()) {
            let armory_response: Result<ArmoryCharacterResponse, serde_json::Error> = serde_json::from_str(&&captures[1]);
            if armory_response.is_err() {
                error!("Error parsing armory response: {:?}", armory_response.err());
                return None;
            }

            let tmp = armory_response.unwrap();
            return Some(tmp);
        }
        return None;
    }

    pub fn check_raid_boss_kills(armory: &ArmoryCharacterResponse, raid_data: &mut BTreeMap<usize, PlayerRaidData>) {
        //info!("Checking raid boss kills for raid IDs: {:?}", settings.required_raids);
        //let mut unkilled_bosses = Vec::new();
        
        if armory.summary.raids.is_empty() {
            warn!("No raid data found for character");
            return;
        }

        for raid in &armory.summary.raids {
            for difficulty in &raid.difficulties {
                for boss in &difficulty.bosses {
                    let raid_id = armory.summary.raids.iter().position(|x| x.name == raid.name).unwrap();
                    let difficulty_id = armory.summary.raids.get(raid_id).unwrap().difficulties.iter().position(|x| x.name == difficulty.name).unwrap();
                    let boss_id = difficulty.bosses.iter().position(|x| x.name == boss.name).unwrap();

                    let boss_data = raid_data.entry(raid_id).or_insert(PlayerRaidData { raid_name: raid.name.clone(), bosses: BTreeMap::new() })
                        .bosses.entry(boss_id).or_insert(PlayerRaidBossData { boss_id, boss_name: boss.name.clone(), difficulties: BTreeMap::new() })
                        .difficulties.entry(difficulty_id).or_insert(PlayerRaidBossDifficultyData { difficulty_id, difficulty_name: difficulty.name.clone(), boss_kill_time: None, killed_before: false });
                    if boss.kill_count > 0 {
                        boss_data.killed_before = true;
                    } else {
                        boss_data.killed_before = false;
                    }
                }
            }
        }
    }
}