use std::collections::BTreeMap;

use regex::Regex;

use crate::{checker::{check_player::PlayerChecker, raid_sheet::PlayerOnlyCheckType}, config::{self, expansion_config::ExpansionRaid, settings::{RequiredRaid, RequiredRaidDifficulty}}};

#[derive(PartialEq)]
pub(crate) enum QuestionState {
    None,
    AskSaved,
    AskSavedBosses,
    AskRaidHelperURL
}

pub struct RaidCheckQuestions {
    pub(crate) state: QuestionState,
    pub(crate) saved_bosses: BTreeMap<i32, RequiredRaid>,
    pub(crate) raid_helper_url: String,
    raid_helper_url_error: bool,
    pub(crate) ignore_url_question: bool,
    pub(crate) player_only: PlayerOnlyCheckType,
    display_raid_id: i32,
    display_difficulty_id: i32,
}

pub enum MatchType {
    RaidPlan(String),
    Event(String),
    ApiV2Event(String),
}

pub fn get_url_match_type(url: &str) -> Option<MatchType> {
    let re_raidplan = Regex::new(r"^https://raid-helper\.dev/raidplan/(\w+)$").unwrap();
    let re_event = Regex::new(r"^https://raid-helper\.dev/event/(\w+)$").unwrap();
    let re_api_v2_event = Regex::new(r"^https://raid-helper\.dev/api/v2/events/(\w+)$").unwrap();

    if let Some(caps) = re_raidplan.captures(url) {
        Some(MatchType::RaidPlan(caps[1].to_string()))
    } else if let Some(caps) = re_event.captures(url) {
        Some(MatchType::Event(caps[1].to_string()))
    } else if let Some(caps) = re_api_v2_event.captures(url) {
        Some(MatchType::ApiV2Event(caps[1].to_string()))
    } else {
        None
    }
}

fn check_raidhelper_url(url: String) -> Option<String> {
    loop {
        let url_type = get_url_match_type(&url);
        match url_type {
            Some(MatchType::RaidPlan(id)) => {
                return Some(format!("https://raid-helper.dev/api/v2/events/{id}"));
            },

            Some(MatchType::Event(id)) => {
                return Some(format!("https://raid-helper.dev/api/v2/events/{id}"));
            },

            Some(MatchType::ApiV2Event(_id)) => {
                return Some(url)
            },

            _ => {
                return None;
            }
        }
    }
}

impl Default for RaidCheckQuestions {
    fn default() -> Self {
        Self {
            state: QuestionState::None,
            saved_bosses: BTreeMap::new(),
            raid_helper_url: String::default(),
            raid_helper_url_error: false,
            ignore_url_question: false,
            player_only: PlayerOnlyCheckType::None,
            display_raid_id: -1,
            display_difficulty_id: 0,
        }
    }
}

impl RaidCheckQuestions {
    pub fn ask_questions(&mut self, ctx: &eframe::egui::Context, expansion_config: &config::expansion_config::ExpansionsConfig, url: Option<String>, is_player: Option<PlayerOnlyCheckType>) -> Option<(String, BTreeMap<i32, RequiredRaid>, PlayerOnlyCheckType)> {
        let mut send_it: Option<(String, BTreeMap<i32, RequiredRaid>, PlayerOnlyCheckType)> = None;
        if url.clone().is_some() {
            self.raid_helper_url = url.clone().unwrap();
            self.ignore_url_question = true;
        }
        
        if is_player.clone().is_some() {
            self.player_only = is_player.unwrap();
        }

        match self.state {
            QuestionState::AskSaved => {
                if expansion_config.latest_expansion.is_none() || expansion_config.latest_expansion.as_ref().unwrap().latest_season.is_none() {
                    egui::Window::new("There are no raids available to check.")
                        .collapsible(false)
                        .resizable(false)
                        .show(ctx, |ui| {
                            ui.label("There are no raids available to check.");
                            if ui.button("OK").clicked() {
                                self.state = QuestionState::AskRaidHelperURL;
                            }
                        });
                    return None;
                }
                else {
                    egui::Window::new("Would you like to check if the character(s) are saved?")
                        .collapsible(false)
                        .resizable(false)
                        .show(ctx, |ui| {
                            ui.label("Would you like to check if the character(s) are saved?");
                            ui.horizontal(|ui| {
                                if ui.button("Yes").on_hover_ui(|ui| {
                                    ui.label("Checking for Saved bosses will check if the user has killed the bosses on the specific difficulties this reset.");
                                }).clicked() {
                                    self.state = QuestionState::AskSavedBosses;
                                };

                                if ui.button("No").on_hover_ui(|ui| {
                                    ui.label("Characters' boss kills will not be checked this reset.");
                                }).clicked() {
                                    self.state = QuestionState::AskRaidHelperURL;
                                    self.display_raid_id = if self.display_raid_id == -1 {
                                        if expansion_config.latest_expansion.is_none() || expansion_config.latest_expansion.as_ref().unwrap().latest_season.is_none() {
                                            -1
                                        } else {
                                            expansion_config.latest_expansion.as_ref().unwrap().latest_season.as_ref().unwrap().raids.last().unwrap().id
                                        }
                                    } else {
                                        self.display_raid_id
                                    };
                                };

                                if ui.button("Cancel").on_hover_ui(|ui| {
                                    ui.label("Cancel the raid check.");
                                }).clicked() {
                                    self.state = QuestionState::None;
                                }
                            });
                        });
                    }
            },

            QuestionState::AskSavedBosses => {

                self.display_raid_id = if self.display_raid_id == -1 {
                    if expansion_config.latest_expansion.is_none() || expansion_config.latest_expansion.as_ref().unwrap().latest_season.is_none() {
                        -1
                    } else {
                        expansion_config.latest_expansion.as_ref().unwrap().latest_season.as_ref().unwrap().raids.last().unwrap().id
                    }
                } else {
                    self.display_raid_id
                };
                
                egui::Window::new("Saved bosses picker")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        if self.saved_bosses.is_empty() || 
                                self.saved_bosses.iter().find(|x| expansion_config.latest_expansion.as_ref().unwrap().find_raid_by_id(*x.0).is_none()).is_some() {
                            let latest_raid = expansion_config.latest_expansion.as_ref().unwrap().latest_season.as_ref().unwrap().raids.last().unwrap().id;
                            self.saved_bosses.clear();
                            self.saved_bosses.insert(latest_raid, RequiredRaid {
                                id: latest_raid,
                                difficulty: BTreeMap::new(),
                            });
                            self.display_difficulty_id = latest_raid;
                        }

                        egui::ComboBox::from_label("Selected raid")
                            .selected_text(format!("{}", expansion_config.latest_expansion.as_ref().unwrap().find_raid_by_id(self.display_raid_id).unwrap_or(&ExpansionRaid::default()).identifier))
                            .show_ui(ui, |ui| {
                                for season in expansion_config.latest_expansion.as_ref().unwrap().seasons.iter() {
                                    ui.label(season.seasonal_identifier.clone());
                                    for raid in season.raids.iter() {
                                        let text_colour = if self.saved_bosses.get(&raid.id).is_some() && self.saved_bosses.get(&raid.id).unwrap().difficulty.iter().any(|d| !d.1.boss_ids.is_empty()) {
                                            egui::Color32::YELLOW
                                        } 
                                        else{ 
                                            egui::Color32::WHITE
                                        };
                                        ui.selectable_value(&mut self.display_raid_id, raid.id, egui::RichText::new(raid.identifier.clone()).color(text_colour));
                                    }
    
                                    if season.seasonal_identifier == expansion_config.latest_expansion.as_ref().unwrap().latest_season.as_ref().unwrap().seasonal_identifier {
                                        break;
                                    }
                                }
                            });

                        if self.saved_bosses.get(&self.display_raid_id).is_none() {
                            self.saved_bosses.insert(self.display_raid_id, RequiredRaid {
                                id: self.display_raid_id,
                                difficulty: BTreeMap::new(),
                            });
                        }

                        egui::ComboBox::from_label("Selected difficulty")
                            .selected_text(format!("{}", expansion_config.latest_expansion.as_ref().unwrap().find_raid_by_id(self.display_raid_id).unwrap_or(&ExpansionRaid::default()).difficulty.get(self.display_difficulty_id as usize).unwrap().difficulty_name))
                            .show_ui(ui, |ui| {
                                for difficulty in expansion_config.latest_expansion.as_ref().unwrap().find_raid_by_id(self.display_raid_id).unwrap_or(&ExpansionRaid::default()).difficulty.iter() {
                                    let text_colour = if self.saved_bosses.get(&self.display_raid_id).is_some() && 
                                        self.saved_bosses.get(&self.display_raid_id).unwrap().difficulty.get(&difficulty.id).is_some() &&
                                        !self.saved_bosses.get(&self.display_raid_id).unwrap().difficulty.get(&difficulty.id).unwrap().boss_ids.is_empty() {
                                        egui::Color32::YELLOW
                                    } 
                                    else{ 
                                        egui::Color32::WHITE
                                    };
                                    ui.selectable_value(&mut self.display_difficulty_id, difficulty.id, egui::RichText::new(difficulty.difficulty_name.clone()).color(text_colour));
                                }
                            });

                        let raid_difficulty = self.saved_bosses.get_mut(&self.display_raid_id).unwrap().difficulty.entry(self.display_difficulty_id).or_insert(RequiredRaidDifficulty {
                            boss_ids: Vec::new(),
                        });

                        ui.horizontal(|ui| {
                            if ui.button("Enable all bosses").on_hover_ui(|ui| {
                                ui.label("Enable all bosses for this raid.");
                            }).clicked() {
                                raid_difficulty.boss_ids.clear();
                                for i in 0..expansion_config.latest_expansion.as_ref().unwrap().find_raid_by_id(self.display_raid_id).unwrap_or(&ExpansionRaid::default()).boss_names.len() {
                                    raid_difficulty.boss_ids.push(i as i32);
                                }
                            };
    
                            if ui.button("Disable all bosses").on_hover_ui(|ui| {
                                ui.label("Disable all bosses for this raid.");
                            }).clicked() {
                                raid_difficulty.boss_ids.clear();
                            };
                        });
                        
                        let mut bid = 0;
                        for boss in expansion_config.latest_expansion.as_ref().unwrap().find_raid_by_id(self.display_raid_id).unwrap_or(&ExpansionRaid::default()).boss_names.iter() {
                            let mut tmp = raid_difficulty.boss_ids.contains(&bid);
                            if ui.checkbox(&mut tmp, boss).changed() {
                                if tmp {
                                    raid_difficulty.boss_ids.push(bid);
                                } else {
                                    raid_difficulty.boss_ids.retain(|&x| x != bid);
                                }
                            }
                            bid += 1;
                        }
                        
                        ui.label("");
                        ui.horizontal(|ui| {
                            if ui.button("Confirm").on_hover_ui(|ui| {
                                ui.label("You will check every character for kills on the selected bosses on the selected difficulty this reset.");
                            }).clicked() {
                                self.state = QuestionState::AskRaidHelperURL;
                            };

                            if ui.button("Cancel").on_hover_ui(|ui| {
                                ui.label("Cancel the raid check.");
                            }).clicked() {
                                self.state = QuestionState::None;
                                self.saved_bosses.clear();
                            }
                        });
                    });
            },

            QuestionState::AskRaidHelperURL => {
                if self.ignore_url_question && self.raid_helper_url.len() > 0 {
                    send_it = Some((self.raid_helper_url.clone(), self.saved_bosses.clone(), self.player_only.clone()));
                    self.saved_bosses.clear();
                    self.raid_helper_url_error = false;
                    self.state = QuestionState::None;
                }
                else {
                    egui::Window::new("Raid Checker")
                        .collapsible(false)
                        .resizable(false)
                        .show(ctx, |ui| {
                        if self.player_only == PlayerOnlyCheckType::Player {
                            ui.label("Please input the character name you would like to check.");
                        }
                        else {
                            ui.label("Please input the raid-helper URL that contains the signed characters you want to check.");
                        }

                        let response = ui.text_edit_singleline(&mut self.raid_helper_url);
                        let pressed_enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                        if self.raid_helper_url_error == true {
                            ui.label(egui::RichText::new("Invalid Raid Helper URL!").color(egui::Color32::RED));
                        }

                        ui.horizontal(|ui| {
                            if ui.button("Confirm").on_hover_ui(|ui| {
                                ui.label("Sign ups on this URL will be checked.");
                            }).clicked() || pressed_enter == true {
                                if self.player_only == PlayerOnlyCheckType::Player {
                                    send_it = Some((self.raid_helper_url.clone(), self.saved_bosses.clone(), self.player_only.clone()));
                                    self.saved_bosses.clear();
                                    self.raid_helper_url_error = false;
                                    self.state = QuestionState::None;
                                }
                                else {
                                    if let Some(url) = check_raidhelper_url(self.raid_helper_url.clone()) {
                                        self.raid_helper_url = url;
                                        
                                        send_it = Some((self.raid_helper_url.clone(), self.saved_bosses.clone(), self.player_only.clone()));
                                        self.saved_bosses.clear();
                                        self.raid_helper_url_error = false;
                                        self.state = QuestionState::None;
                                        
                                    } else {
                                        self.raid_helper_url_error = true;
                                    }
                                }
                            };

                            

                            if ui.button("Cancel").on_hover_ui(|ui| {
                                ui.label("Cancel the raid check.");
                            }).clicked() {
                                self.state = QuestionState::None;
                            }
                        });
                    });
                }
            }
            _ => {}
        }

        send_it
    }

}