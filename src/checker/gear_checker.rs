use std::collections::HashMap;

use tracing::{error, info};

use crate::{checker::armory_checker::{ArmoryCharacter, CharacterGear}, config::{self, expansion_config::{Expansion, ExpansionsConfig, ItemData}, settings::{Settings, SlotSetting}}};

pub struct GearChecker;

impl GearChecker {
    pub fn check_gear(character: &ArmoryCharacter, settings: &config::settings::Settings, expansions: &config::expansion_config::ExpansionsConfig) -> (Vec<String>, Vec<String>, Vec<String>, i32) {
        let mut enchant_vec = Vec::new();
        let mut socket_vec = Vec::new();
        let mut special_item = Vec::new();
        let mut embelishments = 0;

        if character.gear.is_empty() {
            return (vec![String::from("No gear found.")], Vec::new(), Vec::new(), -1);
        }

        let expansion = expansions.latest_expansion.clone().unwrap();
        let gear_slots = character.gear.clone();
        for gear in gear_slots {
            if gear.1.bonus_list.is_some() {
                for bonus in gear.1.bonus_list.clone().unwrap() {
                    if bonus == expansion.gear_embelishment_bonus_id {
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
                        enchant_vec.push(str);
                    }
                }

                let str = Self::check_gear_socket(&expansions, &gear.1, enchantment_slot.unwrap(), &settings);
                if str.len() > 0 {
                    socket_vec.push(str);
                }

                let special = Self::check_special_item(&expansions, &gear.1, enchantment_slot.unwrap(), &settings);
                if special.len() > 0 {
                    special_item.push(special);
                }
            }
        }
        (enchant_vec, socket_vec, special_item, embelishments)
    }


    fn check_enchant_slot(expansion: &Expansion, gear: &CharacterGear, item: &ItemData, settings: &Settings, expansions: &config::expansion_config::ExpansionsConfig) -> String {
        let binding = settings.current_preset.slots.as_array();
        let item_options_opt: Option<&(SlotSetting, &str)> = binding.iter().find(|x| {
            x.1 == item.slot
        });

        let seasonal_item = if expansion.latest_season.is_none() {
            None 
        } else {
            expansion.latest_season.as_ref().unwrap().seasonal_slot_data.iter().find(|x| {
                x.slot == item.slot  || x.sub_slots.iter().find(|y| **y == item.slot).is_some()
            })
        };

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
            let count = gear.sockets.iter().flatten().filter(|s| s.item.is_some()).count() as i32;
            if count < sockets && options.0.warn_if_socket_unfilled == true {
                if bad_str != "" {
                    bad_str += "\n\t";
                }
                bad_str = format!("{}{} has {} socket{} that are not filled with a gem", bad_str, slot_name, sockets - count, if sockets - count > 1 { "s" } else { "" });
            }
        }

        if options.0.require_greater_socket == true {
            if gear.sockets.is_some() && gear.sockets.clone().unwrap().iter().find(|x| x.item.is_some() && slot.greater_socket_item.iter().find(|y| x.item.as_ref().unwrap().id as i32 == **y).is_some()).is_some() {
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
        if expansions.latest_expansion.is_none() {
            error!("Latest expansion is referencing nothing!");
            return String::default();
        }

        let binding = settings.current_preset.slots.as_array();
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

        let seasonal_slot_opt = if expansion.latest_season.is_none() {
            None 
        } else {
            expansion.latest_season.as_ref().unwrap().seasonal_slot_data.iter().find(|x| {
                x.slot == item.slot  || x.sub_slots.iter().find(|y| **y == item.slot).is_some()
            })
        };

        if let Some(slot_options) = enchant_options_opt {
            if let Some(seasonal_item) = seasonal_slot_opt {
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
    
        let binding = settings.current_preset.slots.as_array();
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

    pub fn check_tier_pieces(gear: &HashMap<String, CharacterGear>, expansions: &config::expansion_config::ExpansionsConfig) -> i32 {
        if expansions.latest_expansion.as_ref().unwrap().latest_season.is_none() {
            return -1;
        }
        
        let mut count = 0;
        let binding = expansions.latest_expansion.clone().unwrap().latest_season.clone().unwrap();
        let tier_sets = binding.tier_gear_ids.clone();
        if tier_sets.is_empty() {
            return -1;
        }

        gear.iter().for_each(|x| {
            if tier_sets.iter().any(|y| x.1.id == *y) {
                count += 1;
            }
        });
        count
    }

    pub fn check_pvp_gear(gear: &HashMap<String, CharacterGear>, expansions: &config::expansion_config::ExpansionsConfig) -> bool {
        if expansions.latest_expansion.as_ref().unwrap().latest_season.is_none() {
            return false;
        }

        let binding = expansions.latest_expansion.clone().unwrap().latest_season.clone().unwrap();
        let pvp_bonus_ids = binding.pvp_bonus_ids.clone();
        if pvp_bonus_ids.is_empty() {
            return false;
        }

        gear.iter().any(|x| {
            x.1.bonus_list.as_ref().map_or(false, |bonus_list| bonus_list.iter().any(|bonus| pvp_bonus_ids.contains(bonus)))
        })
    }
}