use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::{Result};
use regex::Regex;
use reqwest::blocking::Client;
use serde::Deserialize;
use tracing::{error, info, warn};

use crate::config::{self, expansion_config::{Expansion, ExpansionsConfig, ItemData}, settings::{Settings, SlotSetting}};

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

    fn check_special_item(
        expansions: &ExpansionsConfig,
        gear: &CharacterGear,
        item: &ItemData,
        settings: &Settings
    ) -> String {
        info!("Checking special item for slot: {}", item.slot);
    
        let binding = settings.slots.as_array();
        let Some((slot_setting, _)) = binding.iter().find(|(_, slot)| *slot == item.slot) else {
            return String::default();
        };
    
        if !slot_setting.require_special_item {
            return String::default();
        }
    
        let slot_matches = |data: &&ItemData| data.slot == item.slot || data.sub_slots.contains(&item.slot);
        let slot_name = gear.inventory_type.clone().gear_type.to_lowercase();
    
        let perform_check = |item_ids: &[i32], item_type: &str, log_message: &str| {
            if item_ids.is_empty() {
                return None;
            }
            
            info!("{}", log_message);
            if item_ids.contains(&gear.id) {
                Some(String::default())
            } else {
                Some(format!("{} does not have a {} special item!", slot_name, item_type)) // Failure.
            }
        };
    
        if let Some(expansion) = &expansions.latest_expansion {
            if let Some(season) = &expansion.latest_season {
                if let Some(seasonal_item) = season.seasonal_slot_data.iter().find(slot_matches) {
                    if let Some(result) = perform_check(&seasonal_item.special_item_id, "seasonal", &format!("Checking seasonal item for slot: {}", item.slot)) {
                        return result;
                    }
                }
            }
        }
    
        if let Some(expansion) = &expansions.latest_expansion {
            if let Some(expansion_item) = expansion.slot_data.iter().find(slot_matches) {
                if let Some(result) = perform_check(&expansion_item.special_item_id, "expansion", &format!("Checking special expansion item for slot: {}", item.slot)) {
                    return result;
                }
            }
        }
    
        if let Some(agnostic_item) = expansions.agnostic_slot_data.iter().find(slot_matches) {
            if let Some(result) = perform_check(&agnostic_item.special_item_id, "agnostic", &format!("Checking special agnostic item for slot: {}", item.slot)) {
                return result;
            }
        }

        String::default()
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