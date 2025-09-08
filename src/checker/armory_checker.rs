use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDateTime, TimeZone, Utc, Weekday};
use regex::Regex;
use reqwest::blocking::Client;
use serde::Deserialize;
use tracing::{error, info, warn};

use crate::config::{self, expansion_config::{Expansion, ExpansionsConfig, ItemData, RaidAchievements, RaidDifficulty}, settings::{RequiredRaid, RequiredRaidDifficulty, Settings, SlotSetting}};

#[allow(dead_code)]
pub struct ArmoryChecker {}

#[derive(serde::Deserialize, Clone, Debug)]
#[allow(dead_code)]
struct GearEnchantment {
    enchantment_id: i32
}

#[derive(serde::Deserialize, Clone, Debug)]
#[allow(dead_code)]
struct GearInventoryType {
    #[serde(alias = "name")]
    _name: String,
    #[serde(alias = "type")]
    gear_type: String
}

#[derive(serde::Deserialize, Clone, Debug)]
struct GearItem {
    #[serde(alias = "id")]
    _id: u64,
}

#[derive(serde::Deserialize, Clone, Debug)]
struct GearSockets {
    #[serde(alias = "item")]
    _item: Option<GearItem>
}

#[derive(serde::Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct CharacterGear {
    bonus_list: Option<Vec<i32>>,
    enchantments: Option<Vec<GearEnchantment>>,
    id: i32,
    inventory_type: GearInventoryType,
    #[serde(alias = "sockets")]
    sockets: Option<Vec<GearSockets>>
}

#[derive(serde::Deserialize, Clone)]
struct CharacterGearContainer {
    /*back: CharacterGear,
    chest: CharacterGear,
    foot: CharacterGear,
    hand: CharacterGear,
    head: CharacterGear,
    leftFinger: CharacterGear,
    leftTrinket: CharacterGear,
    leg: CharacterGear,
    neck: CharacterGear,
    offhand: Option<CharacterGear>,
    rightFinger: CharacterGear,
    rightTrinket: CharacterGear,
    shoulder: CharacterGear,
    waist: CharacterGear,
    weapon: CharacterGear,
    wrist: CharacterGear,*/
}


#[derive(serde::Deserialize, Clone)]
pub struct ArmoryTimestamp {
    #[serde(alias = "epoch")]
    _epoch: i64,
}

#[derive(serde::Deserialize, Clone)]
#[allow(dead_code)]
pub struct ArmoryCharacter {
    #[serde(alias = "averageItemLevel")]
    pub average_item_level: i32,
    pub gear: HashMap<String, CharacterGear>,
    #[serde(alias = "lastUpdatedTimestamp")]
    pub last_updated_timestamp: ArmoryTimestamp
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
    name: String,
    count: i32,
    total: i32,
    bosses: Vec<ArmoryRaidBosses>
}

#[derive(serde::Deserialize,Clone)]
#[allow(dead_code)]
pub struct ArmoryRaids {
    difficulties: Vec<ArmoryRaidDifficulty>,
    name: String
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
struct Achievements {
    #[serde(alias = "accountWide")]
    account_wide: bool,
    description: String,
    id: i32,
    name: String
}

#[derive(serde::Deserialize, Clone)]
#[allow(dead_code)]
struct AchievementSubCategory {
    achievements: Vec<Achievements>,
    id: String,
    name: String
}

#[derive(serde::Deserialize, Clone)]
#[allow(dead_code)]
struct AchievementCategory {
    //#[serde(skip_deserializing)]
    //achievementsList: Option<Vec<String>>,
    subcategories: HashMap<String, AchievementSubCategory>
}

#[derive(serde::Deserialize, Clone)]
#[allow(dead_code)]
struct ArmoryCharacterAchievementResponse {
    #[serde(alias = "achievementCategory")]
    achievement_category: AchievementCategory,
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
struct ArmoryCharacterReputationResponse {
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

    pub fn check_raid_boss_kills(armory: &ArmoryCharacterResponse, settings: &config::settings::Settings) -> Vec<(String, String)> {
        info!("Checking raid boss kills for raid IDs: {:?}", settings.required_raids);
        let mut unkilled_bosses = Vec::new();
        
        if armory.summary.raids.is_empty() {
            warn!("No raid data found for character");
            return unkilled_bosses;
        }

        for check_raid_ids in settings.required_raids.iter() {
            let mut seen = HashSet::new();
            let mut unkilled_raid_bosses = BTreeMap::new();
            let raid_check: Option<&ArmoryRaids> = armory.summary.raids.get(*check_raid_ids.0 as usize);
            if raid_check.is_none() {
                warn!("No raid data found for raid ID: {}", check_raid_ids.0);
                for check_raid_difficulty in check_raid_ids.1.difficulty.iter() {
                    if check_raid_difficulty.1.boss_ids.is_empty() {
                        continue;
                    }

                    todo!("Handle missing raid data stuff.");
                }
                continue;
            }
            let raid_check = raid_check.unwrap();
            let raid_id = check_raid_ids.0;
            let unique_difficulties: Vec<_> = raid_check.difficulties
                .iter()
                .filter(|x| seen.insert(*x))
                .cloned()
                .collect();
            

            for check_raid_difficulty in check_raid_ids.1.difficulty.iter() {
                let difficulty_option = unique_difficulties.get(*check_raid_difficulty.0 as usize);
                if check_raid_difficulty.1.boss_ids.is_empty() || difficulty_option.is_none() {
                    continue;
                }
                let raid_difficulty = difficulty_option.unwrap();
                let mut boss_id = 0;
                for boss in raid_difficulty.bosses.iter() {
                    if check_raid_difficulty.1.boss_ids.iter().find(|x| **x == boss_id).is_some() {
                        if boss.kill_count == 0 {
                            if unkilled_raid_bosses.contains_key(&boss_id) {
                                let existing: &mut (String, Vec<String>) = unkilled_raid_bosses.get_mut(&boss_id).unwrap();
                                existing.1.push(raid_difficulty.name.clone());
                            } else {
                                unkilled_raid_bosses.insert(boss_id, (boss.name.clone(), vec![raid_difficulty.name.clone()]));
                            }                            
                        }
                    }
                    boss_id += 1;
                }
            }
            for (_, (boss_name, difficulties)) in unkilled_raid_bosses.iter() {
                let difficulty_str = difficulties.join(", ");
                unkilled_bosses.push((raid_check.name.clone(), format!("{} ({})", boss_name, difficulty_str)));
            }
        }
        info!("Unkilled bosses found {:?}", unkilled_bosses);
        unkilled_bosses
    }

    pub fn check_gear(armory: &ArmoryCharacterResponse, settings: &config::settings::Settings, expansions: &config::expansion_config::ExpansionsConfig) -> (Vec<String>, Vec<String>, Vec<String>, i32) {
        info!("--- GEAR CHECK ---");
        let mut enchant_vec = Vec::new();
        let mut socket_vec = Vec::new();
        let mut special_item = Vec::new();
        let mut embelishments = 0;

        if armory.character.gear.is_empty() {
            info!("No gear found for character");
            return (vec![String::from("No gear found.")], Vec::new(), Vec::new(), -1);
        }

        let expansion = expansions.latest_expansion.clone().unwrap();
        let gear_slots = armory.character.gear.clone();
        for gear in gear_slots {
            if gear.1.bonus_list.is_some() {
                for bonus in gear.1.bonus_list.clone().unwrap() {
                    if bonus == expansion.gear_embelishment_bonus_id {
                        info!("Found embelishment bonus on gear: {}", gear.1.inventory_type.gear_type);
                        embelishments += 1;
                    }
                }
            }

            let mut enchantment_slot = expansion.slot_data.iter().find(|x| {
                let mut mtch = x.slot == gear.1.inventory_type.gear_type.to_lowercase();
                if mtch == false {
                    mtch = x.sub_slots.iter().find(|y| **y == gear.1.inventory_type.gear_type.to_lowercase()).is_some();
                }
                mtch
            });

            if enchantment_slot.is_none() {
                let target_type = gear.1.inventory_type.gear_type.to_lowercase();
                enchantment_slot = expansion.latest_season.as_ref()
                    .and_then(|season| {
                        season.seasonal_slot_data.iter().find(|ench| {
                            let mut matches = ench.slot == target_type;
                            if !matches {
                                matches = ench.sub_slots.iter().any(|sub_slot_ref| {
                                    sub_slot_ref.as_str() == target_type
                                });
                            }
                            matches
                        })
                    });
            }

            if enchantment_slot.is_some() {

                if (gear.0 == "offhand" && gear.1.inventory_type.gear_type.to_lowercase() == "weapon") || gear.0 != "offhand" {
                    let str = Self::check_enchant_slot(&expansion, &gear.1, enchantment_slot.unwrap(), &settings, expansions);
                    if str.len() > 0 {
                        info!("{str}");
                        enchant_vec.push(str);
                    }
                }

                // Check for sockets (if needed)
                let str = Self::check_gear_socket(&expansions, &gear.1, enchantment_slot.unwrap(), &settings);
                if str.len() > 0 {
                    info!("{str}");
                    socket_vec.push(str);
                }

                let special = Self::check_special_item(&expansions, &gear.1, enchantment_slot.unwrap(), &settings);
                if special.len() > 0 {
                    info!("{special}");
                    special_item.push(special);
                }
            }
        }
        info!("--- END GEAR CHECK ---");
        (enchant_vec, socket_vec, special_item, embelishments)
    }

    fn check_enchant_slot(expansion: &Expansion, gear: &CharacterGear, item: &ItemData, settings: &Settings, expansions: &config::expansion_config::ExpansionsConfig) -> String {
        info!("Checking enchant slot: {}", item.slot);
        let binding = settings.slots.as_array();
        let item_options_opt: Option<&(SlotSetting, &str)> = binding.iter().find(|x| {
            x.1 == item.slot
        });

        let binding = expansion.latest_season.clone().unwrap();
        let seasonal_item = binding.seasonal_slot_data.iter().find(|x| {
            x.slot == item.slot  || x.sub_slots.iter().find(|y| **y == item.slot).is_some()
        });

        let agnostic_item = expansions.agnostic_slot_data.iter().find(|x| {
            x.slot == item.slot || x.sub_slots.iter().find(|y| **y == item.slot).is_some()
        });

        if let Some(item_options) = item_options_opt {
            if item_options.0.require_slot == true && (!item.enchant_ids.is_empty() || (seasonal_item.is_some() && !seasonal_item.unwrap().enchant_ids.is_empty())) 
                && (gear.enchantments.is_none() || gear.enchantments.as_ref().unwrap().is_empty()) {
                return gear.inventory_type.clone().gear_type.to_lowercase() + " is missing an enchant";
            }
    
            if gear.enchantments.is_none() || gear.enchantments.clone().unwrap().is_empty() {
                return String::default();
            }

            let enchant = gear.enchantments.clone().unwrap();
            if item_options.0.require_latest == true {
                if seasonal_item.is_some() && !seasonal_item.unwrap().enchant_ids.is_empty() {
                    info!("Checking seasonal enchant for slot: {}", item.slot);
                    let seasonal_enchant_ids: Vec<i32> = seasonal_item.clone().unwrap().enchant_ids.clone();
                    let seasonal_lesser_enchant_ids = seasonal_item.clone().unwrap().lesser_enchant_ids.clone();

                    if item_options.0.require_greater == true {
                        if enchant.iter().find(|x| seasonal_lesser_enchant_ids.iter().find(|y| x.enchantment_id == **y).is_some()).is_some() {
                            return format!("{} is enchanted with a \"lesser\" version of an enchant", gear.inventory_type.clone().gear_type.to_lowercase());
                        }
                    }

                    if enchant.iter().find(|x| seasonal_enchant_ids.iter().find(|y| x.enchantment_id == **y).is_some()).is_some() {
                        return String::default();
                    } else {
                        return format!("{} is not enchanted with a \"{} {}\" enchant", gear.inventory_type.clone().gear_type.to_lowercase(), expansion.identifier, expansion.latest_season.clone().unwrap().seasonal_identifier);
                    }
                }

                if enchant.iter().find(|x| item.enchant_ids.iter().find(|y| x.enchantment_id == **y ).is_some()).is_some() || 
                    (agnostic_item.is_some() && agnostic_item.unwrap().enchant_ids.iter().find(|y| enchant.iter().find(|x| x.enchantment_id == **y).is_some()).is_some()) {
                    
                } else if !item.enchant_ids.is_empty() {
                    return format!("{} is not enchanted with a \"{}\" enchant", gear.inventory_type.clone().gear_type.to_lowercase(), expansion.name);
                }
            }

            if item_options.0.require_greater == true {
                if enchant.iter().find(|x| item.lesser_enchant_ids.iter().find(|y| x.enchantment_id == **y).is_some()).is_some() ||
                    (agnostic_item.is_some() && agnostic_item.unwrap().lesser_enchant_ids.iter().find(|y| enchant.iter().find(|x| x.enchantment_id == **y).is_some()).is_some()) {
                    return format!("{} is enchanted with a \"lesser\" version of an enchant", gear.inventory_type.clone().gear_type.to_lowercase());
                }
            }
        }
        
        return String::default();
    }
    
    fn gear_socket_check(gear: &CharacterGear, slot: &ItemData, options: &(SlotSetting, &str)) -> String {
        let required_sockets = options.0.require_sockets;
        let mut bad_str = "".to_string();
        let sockets = gear.sockets.as_ref().map_or(0, |s| s.len()) as i32;
        let slot_name = gear.inventory_type.clone().gear_type.to_lowercase();

        if required_sockets > sockets {
            bad_str = format!("{} is missing {} socket{}", slot_name, required_sockets - sockets, if required_sockets - sockets > 1 { "s" } else { "" });
        }
        if gear.sockets.is_some() {    
            let count = gear.sockets.iter().flatten().filter(|s| s._item.is_some()).count() as i32;
            if count < sockets {
                if bad_str != "" {
                    bad_str += "\n\t";
                }
                bad_str = format!("{}{} has {} socket{} that are not filled with a gem", bad_str, slot_name, sockets - count, if sockets - count > 1 { "s" } else { "" });
            }
        }

        if options.0.require_greater_socket == true {
            if gear.sockets.is_some() && gear.sockets.clone().unwrap().iter().find(|x| x._item.is_some() && slot.greater_socket_item.iter().find(|y| x._item.as_ref().unwrap()._id as i32 == **y).is_some()).is_some() {
                return bad_str;
            } else {
                if bad_str != "" {
                    bad_str += "\n\t";
                }
                return format!("{} does not have a greater gem socketed!", slot_name);
            }
        }
        return bad_str;
    }

    fn check_gear_socket(expansions: &ExpansionsConfig, gear: &CharacterGear, item: &ItemData, settings: &Settings) -> String {
        info!("Checking gear socket for slot: {}", item.slot);

        if expansions.latest_expansion.is_none() {
            error!("Latest expansion is referencing nothing!");
            return String::default();
        }

        let binding = settings.slots.as_array();
        let enchant_options_opt = binding.iter().find(|x| {
            x.1 == item.slot
        });

        let expansion = expansions.latest_expansion.as_ref().unwrap();

        let agnostic_slot_opt = expansions.agnostic_slot_data.iter().find(|x| {
            x.slot == item.slot  || x.sub_slots.iter().find(|y| **y == item.slot).is_some()
        });

        let expansion_slot_opt = expansion.slot_data.iter().find(|x| {
            x.slot == item.slot  || x.sub_slots.iter().find(|y| **y == item.slot).is_some()
        });

        let seasonal_slot_opt: Option<&ItemData> = expansion.latest_season.as_ref().unwrap().seasonal_slot_data.iter().find(|x| {
            x.slot == item.slot  || x.sub_slots.iter().find(|y| **y == item.slot).is_some()
        });

        if let Some(slot_options) = enchant_options_opt {
            if let Some(seasonal_item) = seasonal_slot_opt {
                info!("Checking socket for seasonal slot: {}", item.slot);
                if seasonal_item.has_socket == true {
                    let seasonal_sockets = seasonal_item.max_sockets;
                    if seasonal_sockets > 0 {
                        let bad_retval = Self::gear_socket_check(gear, seasonal_item, slot_options);
                        if bad_retval.len() > 0 {
                            return bad_retval;
                        }
                    }
                }
            }

            if let Some(expansion_slot) = expansion_slot_opt {
                info!("Checking socket status for expansion slot: {}", item.slot);
                if expansion_slot.has_socket == true {
                    let sockets = expansion_slot.max_sockets;
                    if sockets > 0 {
                        let bad_retval = Self::gear_socket_check(gear, expansion_slot, slot_options);
                        if bad_retval.len() > 0 {
                            return bad_retval;
                        }
                    }
                }
            }

            if let Some(agnostic_slot) = agnostic_slot_opt {
                info!("Checking socket status for agnostic slot: {}", item.slot);
                if agnostic_slot.has_socket == true {
                    let sockets = agnostic_slot.max_sockets;
                    if sockets > 0 {
                        let bad_retval = Self::gear_socket_check(gear, agnostic_slot, slot_options);
                        if bad_retval.len() > 0 {
                            return bad_retval;
                        }
                    }
                }
            }

            if item.has_socket == true {
                return Self::gear_socket_check(gear, item, slot_options);
            }  
        }
        
        return String::default();
    }

    fn check_special_item(expansions: &ExpansionsConfig, gear: &CharacterGear, item: &ItemData, settings: &Settings) -> String {
        info!("Checking special item for slot: {}", item.slot);
        let binding = settings.slots.as_array();
        let enchant_options_opt = binding.iter().find(|x| {
            x.1 == item.slot
        });

        let agnostic_item = expansions.agnostic_slot_data.iter().find(|x| {
            x.slot == item.slot || x.sub_slots.iter().find(|y| **y == item.slot).is_some()
        });

        let expansion_item = expansions.latest_expansion.as_ref().unwrap().slot_data.iter().find(|x| {
            x.slot == item.slot || x.sub_slots.iter().find(|y| **y == item.slot).is_some()
        });

        let seasonal_item = expansions.latest_expansion.as_ref().unwrap().latest_season.as_ref().unwrap().seasonal_slot_data.iter().find(|x| {
            x.slot == item.slot || x.sub_slots.iter().find(|y| **y == item.slot).is_some()
        });

        if let Some(enchant_options) = enchant_options_opt {
            let slot_name = gear.inventory_type.clone().gear_type.to_lowercase();
            if enchant_options.0.require_special_item == true {
                if seasonal_item.is_some() && !seasonal_item.unwrap().special_item_id.is_empty() {
                    info!("Checking seasonal item for slot: {}", item.slot);
                    let special =  seasonal_item.unwrap().special_item_id.clone();
                    let found = special.iter().find(|&&x| {
                        x == gear.id
                    });

                    if found.is_none() {
                        return format!("{} does not have a seasonal special item!", slot_name);
                    }
                }
                else if expansion_item.is_some() && !expansion_item.unwrap().special_item_id.is_empty() {
                    info!("Checking special expansion item for slot: {}", item.slot);
                    let special =  expansion_item.unwrap().special_item_id.clone();
                    let found = special.iter().find(|&&x| {
                        x == gear.id
                    });

                    if found.is_none() {
                        return format!("{} does not have an expansion special item!", slot_name);
                    }
                }
                else if agnostic_item.is_some() && !agnostic_item.unwrap().special_item_id.is_empty() {
                    info!("Checking special expansion item for slot: {}", item.slot);
                    let special =  agnostic_item.unwrap().special_item_id.clone();
                    let found = special.iter().find(|&&x| {
                        x == gear.id
                    });

                    if found.is_none() {
                        return format!("{} does not have an agnostic special item!", slot_name);
                    }
                }
            }
        }
        
        return String::default();
    }

    fn get_wednesday_reset_timestamp() -> i64 {
        let now = Utc::now();
        let weekday = now.weekday();
    
        let days_to_subtract = match weekday {
            Weekday::Wed => 0,
            _ => (7 + weekday.num_days_from_monday() as i64 - 2) % 7,
        };
    
        let wednesday_date = now.date_naive() - Duration::days(days_to_subtract);
        let wednesday_4am = wednesday_date.and_hms_opt(4, 0, 0).unwrap();
        wednesday_4am.and_utc().timestamp_millis()
    }

    pub fn check_saved_bosses(
        armory: &ArmoryCharacterResponse,
        raid_saved_check: &BTreeMap<i32, RequiredRaid>,
    ) -> Vec<(String, String)> {
        let reset_timestamp = Self::get_wednesday_reset_timestamp() as u64;
    
        raid_saved_check
            .iter()
            .filter_map(|(&raid_id, required_raid)| {
                // Find the corresponding raid data in the armory. If not found, `filter_map` will discard this item.
                armory.summary.raids.get(raid_id as usize).map(|armory_raid| (armory_raid, required_raid))
            })
            .flat_map(|(armory_raid, required_raid)| {
                let mut killed_bosses_in_raid: BTreeMap<usize, (String, Vec<(String, u64)>)> = BTreeMap::new();
    
                for (&difficulty_id, required_difficulty) in &required_raid.difficulty {
                    if required_difficulty.boss_ids.is_empty() {
                        continue;
                    }
    
                    if let Some(armory_difficulty) = armory_raid.difficulties.get(difficulty_id as usize) {
                        for (boss_id, armory_boss) in armory_difficulty.bosses.iter().enumerate() {
                            if required_difficulty.boss_ids.contains(&(boss_id as i32)) {
                                if let Some(timestamp) = armory_boss.last_timestamp {
                                    if timestamp > reset_timestamp {
                                        let entry = killed_bosses_in_raid
                                            .entry(boss_id)
                                            .or_insert_with(|| (armory_boss.name.clone(), Vec::new()));
                                        entry.1.push((armory_difficulty.name.clone(), timestamp / 1000));
                                    }
                                }
                            }
                        }
                    }
                }
                
                killed_bosses_in_raid.into_iter().map(move |(_, (boss_name, difficulties))| {
                    let (diff_names, timestamps): (Vec<_>, Vec<_>) = difficulties.into_iter().unzip();
                    let diff_str = diff_names.join(", ");
                    let last_kill_timestamp = timestamps.last().unwrap_or(&0);
                    let kill_time: DateTime<Utc> = Utc.timestamp_opt(*last_kill_timestamp as i64, 0).unwrap();
                    
                    (
                        armory_raid.name.clone(),
                        format!(
                            "{} ({}) @ {}",
                            boss_name,
                            diff_str,
                            kill_time.format("%A %H:%M")
                        ),
                    )
                })
            })
        .collect()
    }

    pub fn check_aotc(
        url: String,
        armory: &ArmoryCharacterResponse,
        expansions: &config::expansion_config::ExpansionsConfig,
        raid_saved_check_input: &BTreeMap<i32, RequiredRaid>,
    ) -> BTreeMap<i32, (String, RaidProgressStatus)> {
        info!("--- AOTC CHECK ---");

        let raid_saved_check = Self::determine_raids_to_check(expansions, raid_saved_check_input);

        let feats_url = format!("{}/achievements/feats-of-strength", url.trim_end_matches('/'));
        let response_text = Self::fetch_achievements(&feats_url);

        let mut aotc_ce_status = BTreeMap::new();
        if let Some(data) = Self::extract_achievement_data(&response_text) {
            Self::process_achievements(
                &data,
                armory,
                expansions,
                &raid_saved_check,
                &mut aotc_ce_status,
            );
        } else {
            error!("Could not find character profile initial state in response.");
        }

        Self::fill_missing_raids(&raid_saved_check, expansions, &mut aotc_ce_status);

        aotc_ce_status
    }

    fn determine_raids_to_check(
        expansions: &config::expansion_config::ExpansionsConfig,
        input: &BTreeMap<i32, RequiredRaid>,
    ) -> BTreeMap<i32, RequiredRaid> {
        if input.is_empty()
            || input.iter().all(|x| x.1.difficulty.is_empty())
            || input
                .iter()
                .all(|x| x.1.difficulty.iter().all(|y| y.1.boss_ids.is_empty()))
        {
            info!("Specified raid is empty, assuming last raid.");
            if let Some(latest) = &expansions.latest_expansion {
                if let Some(season) = &latest.latest_season {
                    if let Some(last_raid) = season.raids.last() {
                        return BTreeMap::from([(
                            last_raid.id,
                            RequiredRaid {
                                id: last_raid.id,
                                difficulty: BTreeMap::from([(
                                    1,
                                    RequiredRaidDifficulty { boss_ids: vec![0] },
                                )]),
                            },
                        )]);
                    }
                }
            }
        }
        input.clone()
    }

    fn fetch_achievements(url: &str) -> String {
        let client = Client::new();
        client
            .get(url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36",
            )
            .send()
            .and_then(|r| r.text())
            .unwrap_or_else(|e| {
                error!("Failed to fetch achievements: {}", e);
                String::new()
            })
    }

    fn extract_achievement_data(response: &str) -> Option<ArmoryCharacterAchievementResponse> {
        let re = Regex::new(r#"var\s+characterProfileInitialState\s*=\s*(\{.*?\});"#).unwrap();
        re.captures(response)
            .and_then(|cap| serde_json::from_str(&cap[1]).ok())
    }

    fn process_achievements(
        data: &ArmoryCharacterAchievementResponse,
        armory: &ArmoryCharacterResponse,
        expansions: &config::expansion_config::ExpansionsConfig,
        raids_to_check: &BTreeMap<i32, RequiredRaid>,
        aotc_ce_status: &mut BTreeMap<i32, (String, RaidProgressStatus)>,
    ) {
        // Map achievements by ID for quick lookup
        let mut achievement_map = BTreeMap::new();
        for category in &data.achievement_category.subcategories {
            if category.1.id == "raids" {
                for achievement in &category.1.achievements {
                    achievement_map.insert(achievement.id, achievement);
                }
                break;
            }
        }
    
        for (&raid_id, _) in raids_to_check {
            let raid = expansions
                .latest_expansion
                .as_ref()
                .and_then(|e| e.find_raid_by_id(raid_id))
                .expect("Raid not found in expansion data");
            let raid_name = raid.identifier.clone();
    
            let status = if raid.achievements.dependency_id != -1 {
                let raid_summary = armory.summary.raids.get(raid_id as usize);
                Self::end_boss_status(raid_summary)
            } else {
                let achievement = achievement_map.get(&raid.achievements.aotc)
                    .or_else(|| achievement_map.get(&raid.achievements.ce));
                if let Some(ach) = achievement {
                    Self::aotc_ce_status(ach.id, &raid.achievements, armory.summary.raids.get(raid_id as usize))
                } else {
                    RaidProgressStatus::None
                }
            };
    
            aotc_ce_status.entry(raid_id).or_insert((raid_name, status));
        }
    }
    
    fn aotc_ce_status(
        earned_achievement_id: i32,
        achievements: &RaidAchievements,
        raid: Option<&ArmoryRaids>,
    ) -> RaidProgressStatus {
        if raid.is_none() {
            info!("No raid summary! Depending on purely achievement.");
            return if earned_achievement_id == achievements.ce {
                RaidProgressStatus::CuttingEdge(true, false, false)
            } else {
                RaidProgressStatus::Account
            };
        }

        let raid = raid.unwrap();
        let mut has_cutting_edge = false;
    
        if earned_achievement_id == achievements.ce {
            if let Some(mythic) = raid.difficulties.get(3) {
                if let Some(last_boss) = mythic.bosses.last() {
                    if last_boss.kill_count >= 1 {
                        info!("Character has killed mythic end boss.");
                        has_cutting_edge = true;
                    }
                }
            }
        }
    
        if let Some(heroic) = raid.difficulties.get(2) {
            if let Some(last_boss) = heroic.bosses.last() {
                if last_boss.kill_count >= 1 {
                    return if earned_achievement_id == achievements.ce {
                        RaidProgressStatus::CuttingEdge(true, has_cutting_edge, true)
                    } else if earned_achievement_id == achievements.aotc {
                        RaidProgressStatus::Character
                    } else {
                        RaidProgressStatus::None
                    };
                }
            }
        }
    
        if earned_achievement_id == achievements.ce && has_cutting_edge {
            return RaidProgressStatus::CuttingEdge(true, has_cutting_edge, false);
        }
    
        RaidProgressStatus::Account
    }


    fn end_boss_status(
        raid: Option<&ArmoryRaids>
    ) -> RaidProgressStatus {
        if raid.is_none() {
            return RaidProgressStatus::Error;
        }

        let raid = raid.unwrap();
        info!("Checking end boss kill for {}", raid.name);
        let mythic_killed = raid.difficulties.get(3)
            .and_then(|d| d.bosses.last())
            .map(|b| b.kill_count >= 1)
            .unwrap_or(false);
    
        let heroic_killed = raid.difficulties.get(2)
             .and_then(|d| d.bosses.last())
            .map(|b| b.kill_count >= 1)
            .unwrap_or(false);
    
        RaidProgressStatus::EndBossKilled(heroic_killed || mythic_killed, heroic_killed, mythic_killed)
    }

    fn fill_missing_raids(
        raids_to_check: &BTreeMap<i32, RequiredRaid>,
        expansions: &config::expansion_config::ExpansionsConfig,
        aotc_ce_status: &mut BTreeMap<i32, (String, RaidProgressStatus)>,
    ) {
        for (&raid_id, required) in raids_to_check {
            if !aotc_ce_status.contains_key(&raid_id)
                && required
                    .difficulty
                    .iter()
                    .any(|(_, diff)| !diff.boss_ids.is_empty())
            {
                if let Some(raid) = expansions
                    .latest_expansion
                    .as_ref()
                    .and_then(|e| e.find_raid_by_id(raid_id))
                {
                    info!("No AOTC/CE data found for raid {}", raid.identifier);
                    aotc_ce_status.insert(raid_id, (raid.identifier.clone(), RaidProgressStatus::None));
                }
            }
        }
    }

    pub fn check_raid_buff(
        _url: String,
        expansions: &config::expansion_config::ExpansionsConfig,
        raid_saved_check_input: &BTreeMap<i32, RequiredRaid>,
    ) -> Result<BTreeMap<i32, (String, i32, bool, i32, i32)>> {
        info!("Checking for raid buffs");
    
        let latest_expansion = expansions
            .latest_expansion
            .as_ref()
            .context("Latest expansion configuration is missing")?;

        let raid_saved_check = if raid_saved_check_input.values().all(|r| r.difficulty.values().all(|d| d.boss_ids.is_empty())) {
            info!("No valid raid difficulties specified, assuming last raid of the latest season.");
            let latest_raid = latest_expansion
                .latest_season
                .as_ref()
                .and_then(|s| s.raids.last())
                .context("Could not find the latest raid in configuration")?;
            
            BTreeMap::from([(
                latest_raid.id,
                RequiredRaid {
                    id: latest_raid.id,
                    difficulty: BTreeMap::from([(1, RequiredRaidDifficulty { boss_ids: vec![0] })]),
                },
            )])
        } else {
            raid_saved_check_input.clone()
        };
    
        let url = format!("{}/reputation", _url.trim_end_matches('/'));
        let client = Client::new();
        let response_text = client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36")
            .send()
            .context("Failed to send request to armory")?
            .text()
            .context("Failed to read armory response text")?;
    
        let re = Regex::new(r#"var\s+characterProfileInitialState\s*=\s*(\{.*?\});"#)?;
        let js_variable = re
            .captures(&response_text)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str())
            .context("Could not find 'characterProfileInitialState' JSON in the response")?;
        
        let armory_response: ArmoryCharacterReputationResponse = serde_json::from_str(js_variable)
            .context("Failed to deserialize character JSON data")?;
    
        let mut raid_buffs = BTreeMap::new();
        let all_reputations: Vec<_> = armory_response.reputations.reputations
            .iter()
            .flat_map(|cat| cat.reputations.iter())
            .flat_map(|rep| std::iter::once(rep).chain(rep.reputations.iter()))
            .collect();
    
        for (raid_id, raid_config) in &raid_saved_check {
            if raid_config.difficulty.values().all(|d| d.boss_ids.is_empty()) {
                info!("Skipping raid ID: {} as it has no specified boss IDs to check.", raid_id);
                continue;
            }
    
            let raid = latest_expansion.find_raid_by_id(*raid_id).with_context(|| format!("Configuration for raid ID {} not found", raid_id))?;
            let Some(reputation) = &raid.reputation else {
                info!("Raid {} has no reputation assigned, skipping buff check.", raid.identifier);
                raid_buffs.insert(*raid_id, (raid.identifier.clone(), 0, false, 0, 0));
                continue;
            };

            let character_rep_data = all_reputations.iter().find(|&&rep| rep.id == reputation.raid_rep_slug);
            let (current_renown, current_renown_amount) = if let Some(data) = character_rep_data {
                let renown_str = data.standing.as_deref().unwrap_or("Renown 0").split(' ').last().unwrap_or("0");
                let level = renown_str.parse().unwrap_or(0);
                let amount = data.value.unwrap_or(0) as i32;
                info!("Reputation data found for {}: Renown {}, Amount {}", raid.identifier, level, amount);
                (level, amount)
            } else {
                info!("No reputation data found for raid: {}. Assuming Renown 1.", raid.identifier);
                (1, 0)
            };
            
            let start_time = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp_opt(reputation.renown_start, 0).unwrap(), Utc);
            let max_renown = (Utc::now() - start_time).num_weeks() + 2;

            let missing_buff_levels: Vec<_> = reputation.raid_buff_renowns
                .iter()
                .filter(|&&lvl| lvl > current_renown && (lvl as i64) <= max_renown)
                .copied()
                .collect();
            
            if missing_buff_levels.is_empty() {
                info!("No missing buff levels found for {}, current renown: {}, max renown: {}", raid.identifier, current_renown, max_renown);
                continue;
            }

            let next_buff_renown = *missing_buff_levels.first().unwrap();
            let renown_levels_to_gain = next_buff_renown - current_renown;

            let possible = if character_rep_data.is_some() {
                let weekly_cap = reputation.max_renown_value_weekly + current_renown_amount; // Cap includes current progress.
                let points_needed = renown_levels_to_gain * reputation.renown_level_value;
                (points_needed as f32) <= (weekly_cap as f32)
            } else {
                let points_needed = renown_levels_to_gain * reputation.renown_level_value;
                (points_needed as f32) <= (reputation.max_renown_value_weekly as f32)
            };
            
            info!("{} Missing buff renowns: {:?}, possible to get a buff: {possible}", raid.identifier, missing_buff_levels);
            raid_buffs.insert(
                *raid_id,
                (
                    raid.identifier.clone(),
                    missing_buff_levels.len() as i32,
                    possible,
                    reputation.buff_size,
                    reputation.max_renown_value_weekly,
                ),
            );
        }
    
        Ok(raid_buffs)
    }

    pub fn check_tier_pieces(armory: &ArmoryCharacterResponse, expansions: &config::expansion_config::ExpansionsConfig) -> i32 {
        info!("Checking for tier pieces");
        let mut count = 0;
        let binding = expansions.latest_expansion.clone().unwrap().latest_season.clone().unwrap();
        let tier_sets = binding.tier_gear_ids.clone();
        if tier_sets.is_empty() {
            return -1;
        }

        armory.character.gear.iter().for_each(|x| {
            if tier_sets.iter().any(|y| x.1.id == *y) {
                count += 1;
            }
        });
        count
    }
}