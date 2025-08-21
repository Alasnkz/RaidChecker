use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, Datelike, Duration, Local, NaiveDateTime, TimeZone, Utc, Weekday};
use regex::Regex;
use reqwest::blocking::Client;
use serde::Deserialize;
use tracing::{error, info, warn};

use crate::config::{self, expansion_config::{ExpansionEnchants, Expansions}, settings::{EnchantmentSlotSetting, RequiredRaid, Settings}};

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
pub enum AOTCStatus {
    None,
    Account,
    Character,
    CuttingEdge(bool, bool, bool), // Account, Character, Charcter Heroic Kill
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

            let mut enchantment_slot = expansion.gear_enchants.iter().find(|x| {
                let mut mtch = x.slot == gear.1.inventory_type.gear_type.to_lowercase();
                if mtch == false {
                    mtch = x.sub_slots.iter().find(|y| **y == gear.1.inventory_type.gear_type.to_lowercase()).is_some();
                }
                mtch
            });

            if enchantment_slot.is_none() {
                let target_type = gear.1.inventory_type.gear_type.to_lowercase();
                enchantment_slot = expansion.latest_season.as_ref()
                    .and_then(|raid| raid.seasonal_gear.as_ref())
                    .and_then(|gear_vec| {
                        gear_vec.iter().find(|ench| {
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
                let str = Self::check_gear_socket(&expansion, &gear.1, enchantment_slot.unwrap(), &settings);
                if str.len() > 0 {
                    info!("{str}");
                    socket_vec.push(str);
                }

                let special = Self::check_special_item(&expansion, &gear.1, enchantment_slot.unwrap(), &settings);
                if special.len() > 0 {
                    info!("{special}");
                    special_item.push(special);
                }
            }
        }
        info!("--- END GEAR CHECK ---");
        (enchant_vec, socket_vec, special_item, embelishments)
    }

    fn check_enchant_slot(expansion: &Expansions, slot: &CharacterGear, enchants: &ExpansionEnchants, settings: &Settings, expansions: &config::expansion_config::ExpansionsConfig) -> String {
        info!("Checking enchant slot: {}", enchants.slot);
        let binding = settings.enchantments.as_array();
        let enchant_options_opt = binding.iter().find(|x| {
            x.1 == enchants.slot
        });

        let _binding = Vec::new();
        let binding = expansion.latest_season.clone().unwrap();
        let seasonal_item = binding.seasonal_gear.as_ref().unwrap_or(&_binding).iter().find(|x| {
            x.slot == enchants.slot  || x.sub_slots.iter().find(|y| **y == enchants.slot).is_some()
        });

        let agnostic_item = expansions.agnostic_gear_enchants.iter().find(|x| {
            x.slot == enchants.slot || x.sub_slots.iter().find(|y| **y == enchants.slot).is_some()
        });

        if let Some(enchant_options) = enchant_options_opt {
            if enchant_options.0.require_slot == true && (!enchants.enchant_ids.is_empty() || (seasonal_item.is_some() && !seasonal_item.unwrap().enchant_ids.is_empty())) 
                && (slot.enchantments.is_none() || slot.enchantments.clone().unwrap().is_empty()) {
                return slot.inventory_type.clone().gear_type.to_lowercase() + " is missing an enchant";
            }
    
            if slot.enchantments.is_none() || slot.enchantments.clone().unwrap().is_empty() {
                return String::default();
            }

            let enchant = slot.enchantments.clone().unwrap();
            if enchant_options.0.require_latest == true {
                if seasonal_item.is_some() && !seasonal_item.unwrap().enchant_ids.is_empty() {
                    info!("Checking seasonal item for slot: {}", enchants.slot);
                    let seasonal_enchant_ids = seasonal_item.clone().unwrap().enchant_ids.clone();
                    let seasonal_lesser_enchant_ids = seasonal_item.clone().unwrap().lesser_enchant_ids.clone();

                    if enchant_options.0.require_greater == true {
                        if enchant.iter().find(|x| seasonal_lesser_enchant_ids.is_some() && seasonal_lesser_enchant_ids.clone().unwrap().iter().find(|y| x.enchantment_id == **y).is_some()).is_some() {
                            return format!("{} is enchanted with a \"lesser\" version of an enchant", slot.inventory_type.clone().gear_type.to_lowercase());
                        }
                    }

                    if enchant.iter().find(|x| seasonal_enchant_ids.iter().find(|y| x.enchantment_id == **y).is_some()).is_some() {
                        return String::default();
                    } else {
                        return format!("{} is not enchanted with a \"{} {}\" enchant", slot.inventory_type.clone().gear_type.to_lowercase(), expansion.identifier, expansion.latest_season.clone().unwrap().seasonal_identifier);
                    }
                }

                if enchant.iter().find(|x| enchants.enchant_ids.iter().find(|y| x.enchantment_id == **y ).is_some()).is_some() || 
                    (agnostic_item.is_some() && agnostic_item.unwrap().enchant_ids.iter().find(|y| enchant.iter().find(|x| x.enchantment_id == **y).is_some()).is_some()) {
                    //return String::default();
                } else if !enchants.enchant_ids.is_empty() {
                    return format!("{} is not enchanted with a \"{}\" enchant", slot.inventory_type.clone().gear_type.to_lowercase(), expansion.name);
                }
            }

            if enchant_options.0.require_greater == true {
                if enchant.iter().find(|x| enchants.lesser_enchant_ids.is_some() && enchants.lesser_enchant_ids.clone().unwrap().iter().find(|y| x.enchantment_id == **y).is_some()).is_some() {
                    return format!("{} is enchanted with a \"lesser\" version of an enchant", slot.inventory_type.clone().gear_type.to_lowercase());
                }
            }
        }
        
        return String::default();
    }
    
    fn gear_socket_check(slot: &CharacterGear, enchants: &ExpansionEnchants, options: (EnchantmentSlotSetting, &str)) -> String {
        let required_sockets = options.0.require_sockets;
        let mut bad_str = "".to_string();
        let sockets = slot.sockets.as_ref().map_or(0, |s| s.len()) as i32;
        let slot_name = slot.inventory_type.clone().gear_type.to_lowercase();

        if required_sockets > sockets {
            bad_str = format!("{} is missing {} socket{}", slot_name, required_sockets - sockets, if required_sockets - sockets > 1 { "s" } else { "" });
        }
        if slot.sockets.is_some() {    
            let count = slot.sockets.iter().flatten().filter(|s| s._item.is_some()).count() as i32;
            if count < sockets {
                if bad_str != "" {
                    bad_str += "\n\t";
                }
                bad_str = format!("{}{} has {} socket{} that are not filled with a gem", bad_str, slot_name, sockets - count, if sockets - count > 1 { "s" } else { "" });
            }
        }

        if options.0.require_greater_socket == true {
            if slot.sockets.is_some() && slot.sockets.clone().unwrap().iter().find(|x| x._item.is_some() && enchants.greater_socket_item.iter().find(|y| x._item.as_ref().unwrap()._id as i32 == **y).is_some()).is_some() {
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

    fn gear_socket_seasonal_check(slot: &CharacterGear, options: &EnchantmentSlotSetting, seasonal_item: (ExpansionEnchants, &str)) -> String {
        let mut bad_str = "".to_string();
        let sockets = slot.sockets.as_ref().map_or(0, |s| s.len()) as i32;
        let slot_name = slot.inventory_type.clone().gear_type.to_lowercase();
        let required_sockets = options.require_sockets;

        if required_sockets > sockets {
            bad_str = format!("{} is missing {} socket{}", slot_name, required_sockets - sockets, if required_sockets - sockets > 1 { "s" } else { "" });
        }
        if slot.sockets.is_some() {    
            let count = slot.sockets.iter().flatten().filter(|s| s._item.is_some()).count() as i32;
            if count < sockets {
                if bad_str != "" {
                    bad_str += "\n\t";
                }
                bad_str = format!("{}{} has {} socket{} that are not filled with a gem", bad_str, slot_name, sockets - count, if sockets - count > 1 { "s" } else { "" });
            }
        }

        if options.require_greater_socket == true {
            if slot.sockets.is_some() && slot.sockets.clone().unwrap().iter().find(|x| x._item.is_some() && seasonal_item.0.greater_socket_item.iter().find(|y| x._item.as_ref().unwrap()._id as i32 == **y).is_some()).is_some() {
                return bad_str;
            } else {
                if bad_str != "" {
                    bad_str += "\n\t";
                }
                return format!("{}{} does not have a greater gem socketed!", bad_str, slot_name);
            }
        }
        return bad_str;
    }

    fn check_gear_socket(expansion: &Expansions, slot: &CharacterGear, enchants: &ExpansionEnchants, settings: &Settings) -> String {
        info!("Checking gear socket for slot: {}", enchants.slot);
        let binding = settings.enchantments.as_array();
        let enchant_options_opt = binding.iter().find(|x| {
            x.1 == enchants.slot
        });

        let binding = expansion.latest_season.clone().unwrap();
        let _binding = Vec::new();
        let seasonal_item_opt: Option<&ExpansionEnchants> = binding.seasonal_gear.as_ref().unwrap_or(&_binding).iter().find(|x| {
            x.slot == enchants.slot  || x.sub_slots.iter().find(|y| **y == enchants.slot).is_some()
        });

        let mut bad_retval = String::default();
        

        if let Some(enchant_options) = enchant_options_opt {
            if let Some(seasonal_item) = seasonal_item_opt {
                info!("Checking socket for seasonal slot: {}", enchants.slot);
                if seasonal_item.has_socket == true {
                    let seasonal_sockets = seasonal_item.max_sockets;
                    if seasonal_sockets > 0 {
                        bad_retval = Self::gear_socket_seasonal_check(slot, &enchant_options.0, (seasonal_item.clone(), enchants.slot.as_str()));
                        if bad_retval.len() > 0 {
                            return bad_retval;
                        }
                    }
                }
            }

            if enchants.has_socket == true {
                return Self::gear_socket_check(slot, enchants, (enchant_options.0.clone(), enchants.slot.as_str()));
            }  
        }
        
        return String::default();
    }

    fn check_special_item(expansion: &Expansions, slot: &CharacterGear, enchants: &ExpansionEnchants, settings: &Settings) -> String {
        info!("Checking special item for slot: {}", enchants.slot);
        let binding = settings.enchantments.as_array();
        let enchant_options_opt = binding.iter().find(|x| {
            x.1 == enchants.slot
        });

        let binding = Vec::new();
        let seasonal_item = expansion.latest_season.as_ref().unwrap().seasonal_gear.as_ref().unwrap_or(&binding).iter().find(|x| {
            x.slot == enchants.slot || x.sub_slots.iter().find(|y| **y == enchants.slot).is_some()
        });

        if let Some(enchant_options) = enchant_options_opt {

            let slot_name = slot.inventory_type.clone().gear_type.to_lowercase();
            if enchant_options.0.require_special_item == true && seasonal_item.is_some() && seasonal_item.unwrap().special_item_id.is_some() {
                info!("Checking seasonal item for slot: {}", enchants.slot);
                let special =  seasonal_item.unwrap().special_item_id.clone().unwrap();
                let found = special.iter().find(|&&x| {
                    x == slot.id
                });

                if found.is_none() {
                    return format!("{} does not have a special item!", slot_name);
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

    pub fn check_saved_bosses(armory: &ArmoryCharacterResponse, raid_saved_check: &BTreeMap<i32, RequiredRaid>) -> Vec<(String, String)> {
        let reset = Self::get_wednesday_reset_timestamp();
        let mut killed_bosses = Vec::new();
        for check_raid_ids in raid_saved_check.iter() {
            let mut seen = HashSet::new();
            let mut killed_raid_bosses = BTreeMap::new();
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
                        if boss.last_timestamp.is_some() {
                            if boss.last_timestamp.unwrap() > reset.try_into().unwrap() {
                                if killed_raid_bosses.contains_key(&boss_id) {
                                    let existing: &mut (String, (Vec<(String, u64)>)) = killed_raid_bosses.get_mut(&boss_id).unwrap();
                                    existing.1.push((raid_difficulty.name.clone(), boss.last_timestamp.unwrap()));
                                } else {
                                    killed_raid_bosses.insert(boss_id, (boss.name.clone(), vec![(raid_difficulty.name.clone(), boss.last_timestamp.unwrap() / 1000)]));
                                }
                            }                    
                        }
                    }
                    boss_id += 1;
                }
            }
            for (_, (boss_name, difficulties)) in killed_raid_bosses.iter() {
                let (difficulty, timestamp): (Vec<String>, Vec<u64>) = difficulties.iter().cloned().unzip();
                let diff_str = difficulty.join(", ");
                let season_start: DateTime<Utc> = Utc.timestamp_opt(*timestamp.last().unwrap() as i64, 0).unwrap();
                killed_bosses.push((raid_check.name.clone(), format!("{} ({}) @ {}", boss_name, diff_str, season_start.format("%A %H:%M").to_string())));
            }
        }
        killed_bosses
    }

    // TODO: This breaks on windows release builds??
    pub fn check_aotc(_url: String, armory: &ArmoryCharacterResponse, expansions: &config::expansion_config::ExpansionsConfig, raid_saved_check: &BTreeMap<i32, RequiredRaid>) -> BTreeMap<i32, (String, AOTCStatus)> {
        info!("--- AOTC CHECK ---");
        let client = Client::new();
    
        let url = _url.clone().trim_end_matches('/').to_string() + "/achievements/feats-of-strength";
        let __url = url.clone();

        let response = client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36")
            .send().unwrap()
            .text().unwrap();

        let mut aotc_ce_status = BTreeMap::new();
        let re = Regex::new(r#"var\s+characterProfileInitialState\s*=\s*(\{.*?\});"#).unwrap();
        if let Some(captures) = re.captures(&response) {
            info!("Found character profile initial state in response.");
            let js_variable = &captures[1];
            let armory_response: ArmoryCharacterAchievementResponse = serde_json::from_str(&js_variable).unwrap();
            for category in armory_response.achievement_category.subcategories {
                info!("Checking category: {}", category.1.name);
                if category.1.id == "raids" {
                    info!("Found raids category in achievements.");
                    for achievement in category.1.achievements {
                        let mut raid_name = String::default();
                        info!("Checking achievement: {} (ID: {})", achievement.name, achievement.id);
                        let selected_raid = raid_saved_check.iter().find(|x| { 
                            let raid = expansions.latest_expansion.as_ref().unwrap().find_raid_by_id(*x.0).unwrap();
                            raid_name = raid.identifier.clone();
                            raid.aotc_achievement_id == achievement.id || raid.ce_achievement_id == achievement.id
                        });
                        if selected_raid.is_some() {
                            info!("Found AOTC/CE achievement: {} (ID: {})", achievement.name, achievement.id);
                            let selected_raid = selected_raid.unwrap();
                            let raid_summary = armory.summary.raids.get(*selected_raid.0 as usize);
                            let aotc_achievement_id = expansions.latest_expansion.as_ref().unwrap().find_raid_by_id(*selected_raid.0).map_or_else(Default::default, |raid| raid.aotc_achievement_id);
                            let ce_achievement_id = expansions.latest_expansion.as_ref().unwrap().find_raid_by_id(*selected_raid.0).map_or_else(Default::default, |raid| raid.ce_achievement_id);
                            if raid_summary.is_some() {
                                info!("Found raid summary for raid ID: {}", selected_raid.0);
                                let mut char_ce = false;
                                if achievement.id == ce_achievement_id {
                                    info!("Account has Cutting Edge achievement, checking for Mythic last boss kill.");
                                    let mythic_difficulty = raid_summary.unwrap().difficulties.get(3 as usize);
                                    if mythic_difficulty.is_some() && mythic_difficulty.unwrap().bosses.last().unwrap().kill_count >= 1 {
                                        info!("Character has Cutting Edge achievement.");
                                        char_ce = true;
                                    }
                                }

                                let raid_difficulty = raid_summary.unwrap().difficulties.get(2 as usize);
                                if raid_difficulty.is_some() {
                                    info!("Found heroic difficulty for raid ID: {}", selected_raid.0);
                                    if raid_difficulty.unwrap().bosses.last().unwrap().kill_count >= 1 {
                                        info!("Heroic difficulty has last boss killed, character AOTC achieved.");

                                        if achievement.id == ce_achievement_id {
                                            info!("Character CE has killed last boss on heroic.");
                                            aotc_ce_status.insert(*selected_raid.0, (raid_name.clone(), AOTCStatus::CuttingEdge(true, char_ce, true)));
                                        } else {
                                            info!("Character has AOTC achievement.");
                                            aotc_ce_status.insert(*selected_raid.0, (raid_name.clone(), AOTCStatus::Character));
                                        }
                                    }
                                }

                                if achievement.id == ce_achievement_id && char_ce && raid_difficulty.is_none() {
                                    info!("Character has Cutting Edge achievement, but no end boss heroic kill found for character.");
                                    aotc_ce_status.insert(*selected_raid.0, (raid_name.clone(), AOTCStatus::CuttingEdge(true, char_ce, false)));
                                }
                            }
                            
                            if achievement.id == ce_achievement_id && !aotc_ce_status.contains_key(selected_raid.0){
                                info!("Account has Cutting Edge achievement, no end boss heroic kill found for character.");
                                aotc_ce_status.insert(*selected_raid.0, (raid_name.clone(), AOTCStatus::CuttingEdge(true, false, false)));
                            }
                            

                            if !aotc_ce_status.contains_key(selected_raid.0) {
                                info!("Account has AOTC achievement, no end boss heroic kill found for character.");
                                aotc_ce_status.insert(*selected_raid.0, (raid_name.clone(), AOTCStatus::Account));
                            }
                            
                        }
                    }
                    break;
                }
            }
        } else {
            error!("Could not find character profile initial state in response.");
        }
        info!("No AOTC data found for account.");
        
        for (raid_id, _) in raid_saved_check.iter() {
            if !aotc_ce_status.contains_key(&raid_id) {
                let raid = expansions.latest_expansion.as_ref().unwrap().find_raid_by_id(*raid_id).unwrap().identifier.clone();
                info!("No AOTC/CE data found for raid ID: {}", raid_id);
                aotc_ce_status.insert(*raid_id, (raid, AOTCStatus::None));
            }
        }

        aotc_ce_status
    }

    pub fn check_raid_buff(_url: String, expansions: &config::expansion_config::ExpansionsConfig, raid_id: i32) -> (i32, bool, i32, i32) {
        info!("Checking for raid buff");
        let binding = expansions.latest_expansion.clone().unwrap();
        let raid = binding.find_raid_by_id(raid_id);
        if raid.is_none() {
            info!("Could not find raid with id: {}", raid_id);
            return (0, false, 0, 0);
        }

        if raid.clone().unwrap().reputation.is_none() {
            info!("No raid reputation found for this raid.");
            return (0, false, 0, 0);
        }

        let client = Client::new();
    
        let url = _url.clone().trim_end_matches('/').to_string() + "/reputation";
        let __url = url.clone();

        let response = client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36")
            .send().unwrap()
            .text().unwrap();

        let re = Regex::new(r#"var\s+characterProfileInitialState\s*=\s*(\{.*?\});"#).unwrap();

        let mut o_data: Option<ReputationCategory> = None;
        let reputation = raid.unwrap().reputation.clone().unwrap();
        if let Some(captures) = re.captures(&response) {
            let js_variable = &captures[1];
            let armory_response: ArmoryCharacterReputationResponse = serde_json::from_str(&js_variable).unwrap();
            for category in armory_response.reputations.reputations {
                if category.id == binding.reputation_slug {
                    for expansion_rep in category.reputations {
                        if expansion_rep.id == reputation.raid_rep_slug {
                            o_data = Some(expansion_rep.clone());
                        } else if expansion_rep.reputations.len() > 0 {
                            for sub_rep in expansion_rep.reputations {
                                if sub_rep.id == reputation.raid_rep_slug {
                                    o_data = Some(sub_rep.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    
        let time = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp_opt(reputation.renown_start, 0).unwrap(), Utc);
        let now = Utc::now();
        let duration = now - time;
        let weeks = duration.num_weeks() + 1;
        let max_renown = weeks + 1; // Can gain 2 renown on the first week.
        let buff_renowns = reputation.raid_buff_renowns;

        if o_data.is_some() {
            let data = o_data.unwrap();
            info!("Reputation data found: {:?}", data);
            
            let renown = data.standing.unwrap().split(" ").last().unwrap().parse::<i32>().unwrap();
            let renown_amount = data.value.unwrap_or(0) as i32;
            let weekly = reputation.max_renown_value_weekly + renown_amount; // Add our current renown amount to the weekly cap.
            let missing_buff_levels: Vec<i32> = buff_renowns
                .iter()
                .filter(|&&lvl| lvl <= max_renown as i32 && lvl > renown)
                .copied()
                .collect();

            if missing_buff_levels.len() > 0 {
                let first_renown = missing_buff_levels.first().unwrap().clone();
                let diff = first_renown - renown;
                let possible = ((diff * data.max_value.unwrap() as i32) as f32 / weekly as f32) <= 1.0 as f32;
                info!("Missing buff levels: {:?}, possible to get a buff with 5k backup: {}", missing_buff_levels, possible);
                return (missing_buff_levels.len() as i32, possible, reputation.buff_size, reputation.max_renown_value_weekly);
            } else {
                info!("No missing buff levels found, current renown: {}, max renown: {}", renown, max_renown);
            }

        } else {
            info!("No reputation data found for raid: {}", raid_id);
            let renown = 1; // Assume we'll start at 1.
            let missing_buff_levels: Vec<i32> = buff_renowns
                .iter()
                .filter(|&&lvl| lvl <= max_renown as i32 && lvl > renown)
                .copied()
                .collect();

            if missing_buff_levels.len() > 0 {
                let first_renown = missing_buff_levels.first().unwrap().clone();
                let diff = first_renown - renown;
                let possible = (diff as f32 * reputation.renown_level_value as f32 / reputation.max_renown_value_weekly as f32) <= 1.0;
                info!("Missing buff levels: {:?}, possible to get a buff with {} backup: {}", missing_buff_levels, reputation.max_renown_value_weekly, possible);
                return (missing_buff_levels.len() as i32, possible, reputation.buff_size, reputation.max_renown_value_weekly);
            } else {
                info!("No missing buff levels found, current renown: {}, max renown: {}", renown, max_renown);
            }
        }
        (0, false, 0, 0)
    }
}