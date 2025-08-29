use std::{collections::BTreeMap, sync::mpsc::{Receiver, Sender}};

use regex::Regex;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use tracing::info;

use crate::config::{self, realms::RealmJson, settings::RequiredRaid};

use super::{armory_checker::{AOTCStatus, ArmoryChecker}, raid_sheet::{Player, RaidHelperCheckerStatus, RaidHelperUIStatus}};

pub struct PlayerChecker {}

fn converted_name_correct_realm(ourl: String, realms: &RealmJson) -> String {
    info!("Converting name to correct realm slug: {}", ourl);
    let mut url = ourl.to_lowercase();
    for realm in realms.realms.iter() {
        if url.contains(&realm.name) {
            url = url.replace(&realm.name, &realm.slug);
        }
    }
    return url;
}


fn is_valid_name(name: &str) -> bool {
    let re = Regex::new(r"^[\p{L}\-\']+$").unwrap(); // \p{L} matches letters, \- for hyphen, \' for apostrophe
    re.is_match(name)
}

fn process_name(name: &str) -> Option<(String, String)> {
    let cleaned_name = name.replace("/", "-");

    if is_valid_name(&cleaned_name) {
        let parts: Vec<&str> = cleaned_name.split('-').collect();
        
        if parts.len() == 2 {
            let flipped_name = format!("{}/{}", parts[1], parts[0]);
            Some((flipped_name, parts[0].to_string()))
        } else {
            Some((cleaned_name.clone(), cleaned_name.clone()))
        }
    } else {
        None
    }
}

pub fn slug_to_name(slug: &str, realms: &RealmJson) -> Option<String> {
    for realm in realms.realms.iter() {
        if slug == realm.slug {
            info!("Found realm: {} for slug: {}", realm.name, slug);
            return Some(realm.name.clone());
        }
    }
    None
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PlayerData {
    pub discord_id: String,
    pub name: String,
    pub status: String,
    pub unkilled_bosses: Vec<(String, String)>,
    pub bad_gear: Vec<String>,
    pub bad_socket: Vec<String>,
    pub bad_special_item: Vec<String>,
    pub num_embelishments: i32,
    pub ilvl: i32,
    pub saved_bosses: Vec<(String, String)>,
    pub aotc_status: BTreeMap<i32, (String, AOTCStatus)>,
    pub buff_status: BTreeMap<i32, (String, i32, bool, i32, i32)>,
    pub tier_count: i32,
    pub skip_reason: Option<String>,
    pub armory_url: String,
    pub queued: bool,
}

enum SearchPromptResult {
    Url(String),
    Error(String),
    Skipped,
}

impl PlayerChecker {
    pub fn check_player(player: &Player, thread_sender: &Sender<RaidHelperCheckerStatus>, thread_reciever: &Receiver<RaidHelperUIStatus>,
        settings: &config::settings::Settings, expansions: &config::expansion_config::ExpansionsConfig, realms: &config::realms::RealmJson,
        raid_saved_check: &BTreeMap<i32, RequiredRaid>, char_url: Option<String>) -> Option<PlayerData>
    {
        let mut armory_data = None;

        let mut url = String::default();
        if char_url.is_some() {
            url = char_url.unwrap().clone();
        }
        
        if !url.is_empty() {
            armory_data = ArmoryChecker::check_armory(&url.clone());
        }

        // Check to see if the character's character is a real one.
        let processed_name = process_name(&player.name);
        let is_some = processed_name.is_some();
        if let Some(name) = processed_name {
            if name.0.contains("/") && armory_data.is_none() {
                let proper_url = format!("/en-gb/character/eu/{}/", converted_name_correct_realm(name.0.clone(), realms));
                let full_url = format!("https://worldofwarcraft.blizzard.com{}", proper_url.to_lowercase());
                url = full_url;
                armory_data = ArmoryChecker::check_armory(&url.clone());
            } 

            if armory_data.is_none() {
                let search_response = Self::search_prompt(&name.1.clone(), Some(player), thread_sender, thread_reciever);
                match search_response {
                    SearchPromptResult::Url(search_url) => {
                        url = search_url;
                        armory_data = ArmoryChecker::check_armory(&url.clone());
                    },

                    SearchPromptResult::Skipped => {
                        return Some(PlayerData {
                            discord_id: player.userId.clone(),
                            name: player.name.clone(),
                            status: player.status.clone(),
                            unkilled_bosses: Vec::new(),
                            bad_gear: Vec::new(),
                            bad_socket: Vec::new(),
                            bad_special_item: Vec::new(),
                            num_embelishments: -1,
                            ilvl: 0,
                            saved_bosses: Vec::new(),
                            aotc_status: BTreeMap::new(),
                            buff_status: BTreeMap::new(),
                            tier_count: -1,
                            skip_reason: Some("Skipped by user.".to_owned()),
                            armory_url: "".to_owned(),
                            queued: player.status.to_lowercase() != "primary" || player.className.to_lowercase() == "bench" 
                        });
                    },

                    _ => {}
                }
            }
        }

        if armory_data.is_none() && is_some {
            // Add a thing to say this was auto searched if it auto selects?
            let search_response = Self::search_prompt(&player.name, Some(player), thread_sender, thread_reciever);
            match search_response {
                SearchPromptResult::Url(search_url) => {
                    url = search_url;
                    armory_data = ArmoryChecker::check_armory(&url.clone());
                },

                SearchPromptResult::Skipped => {
                    return Some(PlayerData {
                        discord_id: player.userId.clone(),
                        name: player.name.clone(),
                        status: player.status.clone(),
                        unkilled_bosses: Vec::new(),
                        bad_gear: Vec::new(),
                        bad_socket: Vec::new(),
                        bad_special_item: Vec::new(),
                        num_embelishments: -1,
                        ilvl: 0,
                        saved_bosses: Vec::new(),
                        aotc_status: BTreeMap::new(),
                        buff_status: BTreeMap::new(),
                        tier_count: -1,
                        skip_reason: Some("Skipped by user.".to_owned()),
                        armory_url: "".to_owned(),
                        queued: player.status.to_lowercase() != "primary" || player.className.to_lowercase() == "bench"
                    });
                },

                _ => return None
            }
        }

        if armory_data.is_none() {
            let name = Self::prompt_for_name(Some(player), thread_sender, thread_reciever);
            if name.is_some() {
                let search_response = Self::search_prompt(&name.clone().unwrap(), Some(player), thread_sender, thread_reciever);
                match search_response {
                    SearchPromptResult::Url(search_url) => {
                        url = search_url;
                        armory_data = ArmoryChecker::check_armory(&url.clone());
                    },

                    SearchPromptResult::Skipped => {
                        return Some(PlayerData {
                            discord_id: player.userId.clone(),
                            name: player.name.clone(),
                            status: player.status.clone(),
                            unkilled_bosses: Vec::new(),
                            bad_gear: Vec::new(),
                            bad_socket: Vec::new(),
                            bad_special_item: Vec::new(),
                            num_embelishments: -1,
                            ilvl: 0,
                            saved_bosses: Vec::new(),
                            aotc_status: BTreeMap::new(),
                            buff_status: BTreeMap::new(),
                            tier_count: -1,
                            skip_reason: Some("Skipped by user.".to_owned()),
                            armory_url: "".to_owned(),
                            queued: player.status.to_lowercase() != "primary" || player.className.to_lowercase() == "bench"
                        });
                    }
                    _ => return None
                }
            } else {
                return None;
            }
        }

        info!("------------------- Checking player {} -------------------", player.name);
        let data = armory_data.unwrap();
        let unkilled_bosses = ArmoryChecker::check_raid_boss_kills(&data, settings);
        let (bad_enchant_gear, bad_socket_gear, bad_special_item, embelishments) = ArmoryChecker::check_gear(&data, settings, expansions);
        info!("Character has {} ilvl", data.character.average_item_level);
        let ilvl = data.character.average_item_level;
        let saved_bosses = ArmoryChecker::check_saved_bosses(&data, &raid_saved_check);
        let aotc_report = ArmoryChecker::check_aotc(url.clone(), &data, expansions, &raid_saved_check);
        info!("--- END AOTC CHECK ---");
        let buff_status = ArmoryChecker::check_raid_buff(url.clone(), expansions, &raid_saved_check);
        let tier_count = ArmoryChecker::check_tier_pieces(&data, expansions);
        info!("-------------------- Finished checking player {} -------------------", player.name);
        Some(PlayerData {
            discord_id: player.userId.clone(),
            name: player.name.clone(),
            status: player.status.clone(),
            unkilled_bosses: unkilled_bosses,
            bad_gear: bad_enchant_gear,
            bad_socket: bad_socket_gear,
            bad_special_item: bad_special_item,
            num_embelishments: embelishments,
            ilvl: ilvl,
            saved_bosses: saved_bosses,
            aotc_status: aotc_report,
            buff_status: buff_status,
            tier_count: tier_count,
            skip_reason: None,
            armory_url: url.clone(),
            queued: player.status.to_lowercase() != "primary" || player.className.to_lowercase() == "bench"
        })
    }

    fn prompt_for_name(player: Option<&Player>, thread_sender: &Sender<RaidHelperCheckerStatus>, thread_reciever: &Receiver<RaidHelperUIStatus>) -> Option<String> {
        if player.is_some() {
            let _ = thread_sender.send(RaidHelperCheckerStatus::QuestionStringSkip(format!("Could not find {} (signed as spec {}) - please input a name for this character...", player.unwrap().name, player.unwrap().specName.clone().unwrap_or("Unknown".to_string())))).unwrap();
        } else {
            let _ = thread_sender.send(RaidHelperCheckerStatus::QuestionStringSkip(format!("Please input a name for this character..."))).unwrap();
        }
        
        match thread_reciever.recv().unwrap() {
            RaidHelperUIStatus::AnswerStringSkip(answer) => {
                answer
            },
            _ => None
        }
    }

    fn search_prompt(name: &String, player: Option<&Player>, thread_sender: &Sender<RaidHelperCheckerStatus>, thread_reciever: &Receiver<RaidHelperUIStatus>) -> SearchPromptResult {
        let url = format!("https://worldofwarcraft.blizzard.com/en-gb/search?q={}", name);
        let client = Client::new();
        let response = client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36")
            .send();
        
        if response.is_err() {
            return SearchPromptResult::Error("Could not fetch the search results.".to_string());
        }

        let text = response.unwrap().text();
        if text.is_err() {
            return SearchPromptResult::Error("Could not fetch the search results.".to_string());
        }

        let mut chars = Vec::new();
        let document = Html::parse_document(&text.unwrap());
        let link_selector = Selector::parse("a.Link.Character").unwrap();
 
        for element in document.select(&link_selector) {
            let href = element.value().attr("href").unwrap_or("");
            let name_selector = Selector::parse(".Character-name").unwrap();
            let level_selector = Selector::parse(".Character-level").unwrap();
            let realm_selector = Selector::parse(".Character-realm").unwrap();

            let name = element.select(&name_selector).next().map(|n| n.text().collect::<Vec<_>>().join(" ")).unwrap_or_default();
            let level = element.select(&level_selector).next().map(|l| l.text().collect::<Vec<_>>().join(" ")).unwrap_or_default();
            let realm = element.select(&realm_selector).next().map(|r| r.text().collect::<Vec<_>>().join(" ")).unwrap_or_default();

            let fixed_href = ("https://worldofwarcraft.blizzard.com/en-gb".to_owned() + href).trim_end_matches('/').to_string();
            chars.push((format!("{} {}, level {}", name, realm, level), fixed_href.to_string()));
        }

        if chars.len() == 1 {
            // TODO: Automatic selection alert!
            return SearchPromptResult::Url(chars.last().unwrap().1.clone());
        } else if chars.is_empty() {
            let name = Self::prompt_for_name(player, thread_sender, thread_reciever);
            if name.is_some() {
                return Self::search_prompt(&name.clone().unwrap(), player, thread_sender, thread_reciever);
            }
            return SearchPromptResult::Skipped;
        }

        let spec = match player {
            Some(p) => Some(p.specName.clone().unwrap()),
            None => None
        };
        let _ = thread_sender.send(RaidHelperCheckerStatus::Search((name.clone(), spec, chars))).unwrap();
        let name_url = match thread_reciever.recv().unwrap() {
            RaidHelperUIStatus::SearchResponse(answer) => {
                answer
            },

            RaidHelperUIStatus::SearchResponseNewName() => {
                let name = Self::prompt_for_name(None, thread_sender, thread_reciever);
                if name.is_some() {
                    return Self::search_prompt(&name.clone().unwrap(), player, thread_sender, thread_reciever);
                }
                None
            },

            RaidHelperUIStatus::SearchResponseSkip() => {
                return SearchPromptResult::Skipped;
            }
            _ => None
        };

        if !name_url.is_some() {
            return SearchPromptResult::Skipped;
        }

        if name_url.is_some() {
            return SearchPromptResult::Url(name_url.unwrap().1);
        }
        
        SearchPromptResult::Url(name_url.unwrap().1)
    }
}
