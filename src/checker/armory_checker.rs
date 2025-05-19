use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Datelike, Duration, Local, NaiveDateTime, TimeZone, Utc, Weekday};
use regex::Regex;
use reqwest::blocking::Client;
use serde::Deserialize;

use crate::config::{self, expansion_config::{ExpansionEnchants, Expansions}, settings::Settings};

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
    enchantments: Option<Vec<GearEnchantment>>,
    inventory_type: GearInventoryType,
    #[serde(alias = "sockets")]
    _sockets: Option<Vec<GearSockets>>
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
    difficulties: Vec<ArmoryRaidDifficulty>
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
    Error
}

impl ArmoryChecker {
    pub fn check_armory(name_url: &str) -> Option<ArmoryCharacterResponse> {
        let client = Client::new();
        let response = client
            .get(name_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3")
            .send();

        if response.is_err() {
            println!("Error getting armory response: {:?}", response.err());
            return None;
        }

        let text = response.unwrap().text();
        if text.is_err() {
            println!("Error getting armory response (text): {:?}", text.err());
            return None;
        }
        let re = Regex::new(r#"var\s+characterProfileInitialState\s*=\s*(\{.*?\});"#).unwrap();
        if let Some(captures) = re.captures(&text.unwrap()) {
            let armory_response: Result<ArmoryCharacterResponse, serde_json::Error> = serde_json::from_str(&&captures[1]);
            if armory_response.is_err() {
                println!("Error parsing armory response: {:?}", armory_response.err());
                return None;
            }

            let tmp = armory_response.unwrap();
            println!("Armory response: {:?}", tmp.clone().character.average_item_level);
            return Some(tmp);
        }
        return None;
    }

    pub fn check_raid_boss_kills(armory: &ArmoryCharacterResponse, settings: &config::settings::Settings) -> Vec<String> {
        let mut unkilled_bosses = Vec::new();
        let raid_check = armory.summary.raids.get(settings.raid_id as usize);

        if let Some(raid) = raid_check {
            let mut seen = HashSet::new();
            let unique_difficulties: Vec<_> = raid.difficulties
                .iter()
                .filter(|x| seen.insert(*x))
                .cloned()
                .collect();
            let difficulty_option = unique_difficulties.get(settings.raid_difficulty as usize);

            if let Some(raid_difficulty) = difficulty_option {
                let mut raid_boss_id = 0;
                for boss in raid_difficulty.bosses.clone() {
                    if settings.raid_difficulty_boss_id_kills.iter().find(|x| **x == raid_boss_id).is_some() {
                        if boss.kill_count == 0 {
                            unkilled_bosses.push(format!("{} ({})", boss.name, raid_difficulty.name));
                        }
                    }
                    raid_boss_id += 1;
                }
            } else {
                println!("Could not find difficulty for this raid.");
            }
        } else {
            println!("Could not find raid.");
        }

        println!("Unkilled bosses: {:?}", unkilled_bosses);
        unkilled_bosses
    }

    pub fn check_gear(armory: &ArmoryCharacterResponse, settings: &config::settings::Settings, expansions: &config::expansion_config::ExpansionsConfig) -> Vec<String> {
        let mut enchant_vec = Vec::new();
        if armory.character.gear.is_empty() {
            return vec![String::from("No gear found.")];
        }

        let expansion = expansions.latest_expansion.clone().unwrap();
        let gear_slots = armory.character.gear.clone();
        for gear in gear_slots {
            let enchantment_slot = expansion.gear_enchants.iter().find(|x| {
                let mut mtch = x.slot == gear.1.inventory_type.gear_type.to_lowercase();
                if mtch == false {
                    mtch = x.sub_slots.iter().find(|y| **y == gear.1.inventory_type.gear_type.to_lowercase()).is_some();
                }
                mtch
            });

            if enchantment_slot.is_some() {

                if (gear.0 == "offhand" && gear.1.inventory_type.gear_type.to_lowercase() == "weapon") || gear.0 != "offhand" {
                    let str = Self::check_enchant_slot(&expansion, &gear.1, enchantment_slot.unwrap(), &settings);
                    if str.len() > 0 {
                        enchant_vec.push(str);
                    }
                }
            }

            // TODO: Check for sockets
        }
        enchant_vec
    }

    fn check_enchant_slot(expansion: &Expansions, slot: &CharacterGear, enchants: &ExpansionEnchants, settings: &Settings) -> String {
        let binding = settings.enchantments.as_array();
        let enchant_options_opt = binding.iter().find(|x| {
            x.1 == enchants.slot
        });

        if let Some(enchant_options) = enchant_options_opt {
            if enchant_options.0.require_slot == true && (slot.enchantments.is_none() || slot.enchantments.clone().unwrap().is_empty()) {
                return slot.inventory_type.clone().gear_type.to_lowercase() + " is missing an enchant";
            }
    
            if slot.enchantments.is_none() {
                return String::default();
            }
    
            let enchant = slot.enchantments.clone().unwrap();
            if enchant_options.0.require_latest == true {
                if enchant.iter().find(|x| enchants.enchant_ids.iter().find(|y| x.enchantment_id == **y ).is_some()).is_some() {
                    //return String::default();
                } else {
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

    fn check_raid_kills(armory: &ArmoryCharacterResponse, raid_id: i32, raid_difficulty: i32, boss_kills: &Vec<i32>) -> (Vec<ArmoryRaidBosses>, String) {
        let raid_summary = armory.summary.raids.get(raid_id as usize);
        if raid_summary.is_none() {
            return (Vec::new(), String::default())
        }

        let mut seen = HashSet::new();
        let unique_difficulties: Vec<_> = raid_summary.unwrap().difficulties
            .iter()
            .filter(|x| seen.insert(*x))
            .cloned()
            .collect();
        let raid_difficulty = unique_difficulties.get(raid_difficulty as usize);

        if raid_summary.is_none() {
            return (Vec::new(), String::default())
        }

        let selected_boss_data: Vec<_> = raid_difficulty.unwrap()
            .bosses
            .iter()
            .enumerate()
            .filter(|(index, _)| boss_kills.contains(&(*index as i32)))
            .map(|(_, boss_data)| boss_data.clone())
            .collect();

        let reset = Self::get_wednesday_reset_timestamp();
        let mut saved_bosses = Vec::new();
        for boss in selected_boss_data {
            if boss.last_timestamp.is_some() {
                if boss.last_timestamp.unwrap() > reset.try_into().unwrap() {
                    saved_bosses.push(boss.clone());
                }
            }
        }

        (saved_bosses, raid_difficulty.unwrap().name.clone())
    }

    pub fn check_saved_bosses(armory: &ArmoryCharacterResponse, raid_id: i32, raid_difficulty: i32, boss_kills: &Vec<i32>, check_saved_prev_difficulty: bool) -> Vec<String> {
        let saved_kills = Self::check_raid_kills(armory, raid_id, raid_difficulty, boss_kills); 
        let prev_diff_saved_kills = if check_saved_prev_difficulty && raid_difficulty > 1 {
            Self::check_raid_kills(armory, raid_id, raid_difficulty - 1, boss_kills)
        } else {
            (Vec::new(), String::default())
        };

        let mut boss_map: HashMap<String, (usize, Option<u64>, Vec<String>)> = HashMap::new();
        let mut order_counter = 0;
    
        // Insert from saved_kills
        for boss in &saved_kills.0 {
            let entry = boss_map.entry(boss.name.clone()).or_insert_with(|| {
                let idx = order_counter;
                order_counter += 1;
                (idx, boss.last_timestamp, vec![saved_kills.1.clone()])
            });
    
            if !entry.2.contains(&saved_kills.1) {
                entry.2.push(saved_kills.1.clone());
            }
        }

        for boss in &prev_diff_saved_kills.0 {
            let entry = boss_map.entry(boss.name.clone()).or_insert_with(|| {
                let idx = order_counter;
                order_counter += 1;
                (idx, boss.last_timestamp, vec![prev_diff_saved_kills.1.clone()])
            });
    
            if entry.1.is_none() {
                entry.1 = boss.last_timestamp;
            }
    
            if !entry.2.contains(&prev_diff_saved_kills.1) {
                entry.2.push(prev_diff_saved_kills.1.clone());
            }
        }
    
        let mut merged: Vec<_> = boss_map
            .into_iter()
            .filter_map(|(name, (order, timestamp_opt, mut difficulties))| {
                timestamp_opt.map(|ts| {
                    difficulties.dedup();
                    let diff_str = difficulties.join(", ");
                    let datetime: DateTime<Local> = Local.timestamp_opt((ts / 1000) as i64, 0).unwrap();

                    let formatted = datetime.format("%A @ %H:%M").to_string();
                    (order, format!("{name} ({diff_str}) killed {}", formatted))
                })
            })
            .collect();
    
        // Sort by first-seen order
        merged.sort_by_key(|(order, _)| *order);
    
        merged.into_iter().map(|(_, line)| line).collect()
    }

    pub fn check_aotc(_url: String, armory: &ArmoryCharacterResponse, expansions: &config::expansion_config::ExpansionsConfig, raid_id: i32) -> AOTCStatus {
        let binding = expansions.latest_expansion.clone().unwrap();
        let raid = binding.raids.get(raid_id as usize);
        if raid.is_none() {
            println!("Failed to find raid ID, latext_expansion {:?}, last raid {:?}, raid_id: {:?}", expansions.latest_expansion_identifier, binding.raids.last(), raid_id);
            return AOTCStatus::Error;
        }
        let achievement_id = raid.unwrap().aotc_achievement_id;

        let client = Client::new();
    
        let url = _url.clone().trim_end_matches('/').to_string() + "/achievements/feats-of-strength";
        let __url = url.clone();

        let response = client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3")
            .send().unwrap()
            .text().unwrap();
    
        let mut account_aotc = false;
        let re = Regex::new(r#"var\s+characterProfileInitialState\s*=\s*(\{.*?\});"#).unwrap();
        if let Some(captures) = re.captures(&response) {
            let js_variable = &captures[1];
            let armory_response: ArmoryCharacterAchievementResponse = serde_json::from_str(&js_variable).unwrap();
            for category in armory_response.achievement_category.subcategories {
                if category.1.id == "raids" {
                    for achievement in category.1.achievements {
                        if achievement.id == achievement_id {
                            account_aotc = true;
                            break;
                        }
                    }
                }
            }
        }
            
        let mut ret = AOTCStatus::None;
        if account_aotc {
            ret = AOTCStatus::Account;
            let raid_summary = armory.summary.raids.get(raid_id as usize);
            if raid_summary.is_some() {
                let raid_difficulty = raid_summary.unwrap().difficulties.get(2 as usize);
                if raid_difficulty.is_some() {
                    if raid_difficulty.unwrap().bosses.last().unwrap().kill_count >= 1 {
                        ret = AOTCStatus::Character;
                   }
                }
            }
        }
        ret
    }

    pub fn check_raid_buff(_url: String, expansions: &config::expansion_config::ExpansionsConfig, raid_id: i32) -> (i32, bool) {
        let binding = expansions.latest_expansion.clone().unwrap();
        let raid = binding.raids.get(raid_id as usize);
        if raid.is_none() {
            println!("Failed to find raid ID, latext_expansion {:?}, last raid {:?}, raid_id: {:?}", expansions.latest_expansion_identifier, binding.raids.last(), raid_id);
            return (0, false);
        }

        if raid.clone().unwrap().reputation.is_none() {
            println!("No raid reputation found for this raid.");
            return (0, false);
        }

        let client = Client::new();
    
        let url = _url.clone().trim_end_matches('/').to_string() + "/reputation";
        let __url = url.clone();

        let response = client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3")
            .send().unwrap()
            .text().unwrap();

        let re = Regex::new(r#"var\s+characterProfileInitialState\s*=\s*(\{.*?\});"#).unwrap();

        let mut data: Option<ReputationCategory> = None;
        let reputation = raid.unwrap().reputation.clone();
        if let Some(captures) = re.captures(&response) {
            let js_variable = &captures[1];
            let armory_response: ArmoryCharacterReputationResponse = serde_json::from_str(&js_variable).unwrap();
            for category in armory_response.reputations.reputations {
                if category.id == binding.reputation_slug {
                    for expansion_rep in category.reputations {
                        if expansion_rep.id == reputation.clone().unwrap().raid_rep_slug {
                            data = Some(expansion_rep.clone());
                        } else if expansion_rep.reputations.len() > 0 {
                            for sub_rep in expansion_rep.reputations {
                                if sub_rep.id == reputation.clone().unwrap().raid_rep_slug {
                                    data = Some(sub_rep.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    
        if data.is_some() {
            let time = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp_opt(reputation.clone().unwrap().renown_start, 0).unwrap(), Utc);
            let now = Utc::now();
            let duration = now - time;
            let weeks = duration.num_weeks() + 1;
            let max_renown = weeks + 1; // Can gain 2 renown on the first week.

            let buff_renowns = reputation.clone().unwrap().raid_buff_renowns;
            let renown = data.clone().unwrap().standing.unwrap().split(" ").last().unwrap().parse::<i32>().unwrap();
            let missing_buff_levels: Vec<i32> = buff_renowns
                .iter()
                .filter(|&&lvl| lvl <= max_renown as i32 && lvl > renown)
                .copied()
                .collect();

            if missing_buff_levels.len() > 0 {
                // Figure out if we can get the buff with 5k backup
                let first_renown = missing_buff_levels.first().unwrap().clone();
                let diff = first_renown - renown;
                let rep_data = data.clone().unwrap();
                let possible = ((diff * rep_data.max_value.unwrap() as i32) as f32 / reputation.clone().unwrap().max_renown_value_weekly as f32) < 1.0 as f32;
                return (missing_buff_levels.len() as i32, possible);
            }
        }
        (0, false)
    }
}