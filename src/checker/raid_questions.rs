use regex::Regex;

use crate::config;

#[derive(PartialEq)]
pub(crate) enum QuestionState {
    None,
    AskSaved,
    AskSavedBosses,
    AskRaidHelperURL
}

pub struct RaidCheckQuestions {
    pub(crate) state: QuestionState,
    pub(crate) raid_id: i32,
    pub(crate) difficulty_id: i32,

    pub(crate) prev_raid_id: i32,
    pub(crate) saved_bosses: Vec<i32>,
    raid_helper_url: String,
    raid_helper_url_error: bool,
    ahead_of_the_curve: bool,
    pub(crate) ignore_url_question: bool,
    pub(crate) previous_difficulty: bool,
    pub(crate) player_only: bool
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
            raid_id: -1,
            difficulty_id: 3,
            prev_raid_id: 0,
            saved_bosses: Vec::new(),
            raid_helper_url: String::default(),
            raid_helper_url_error: false,
            ahead_of_the_curve: false,
            ignore_url_question: false,
            previous_difficulty: true,
            player_only: false
        }
    }
}

impl RaidCheckQuestions {
    pub fn ask_questions(&mut self, ctx: &eframe::egui::Context, expansion_config: &config::expansion_config::ExpansionsConfig, url: Option<String>, is_player: Option<bool>) -> Option<(String, bool, i32, i32, Vec<i32>, bool, bool)> {
        let mut send_it: Option<(String, bool, i32, i32, Vec<i32>, bool, bool)> = None;
        if url.clone().is_some() {
            self.raid_helper_url = url.clone().unwrap();
            self.ignore_url_question = true;
        }
        
        if is_player.clone().is_some() {
            self.player_only = is_player.unwrap();
        }

        match self.state {
            QuestionState::AskSaved => {
                egui::Window::new("Would you like to check if the character(s) are saved?")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("Would you like to check if the character(s) are saved?");
                        ui.horizontal(|ui| {
                            if ui.button("Yes").on_hover_ui(|ui| {
                                ui.label("Checking for Saved bosses will check if the user has killed the bosses on the specific difficulty this reset.");
                            }).clicked() {
                                self.state = QuestionState::AskSavedBosses;
                            };

                            if ui.button("No").on_hover_ui(|ui| {
                                ui.label("Characters' boss kills will not be checked this reset.");
                            }).clicked() {
                                self.state = QuestionState::AskRaidHelperURL;
                                self.raid_id = if self.raid_id == -1 {
                                    expansion_config.latest_expansion.as_ref().unwrap().raids.iter().last().unwrap().id
                                } else {
                                    self.raid_id
                                };
                            };

                            if ui.button("Cancel").on_hover_ui(|ui| {
                                ui.label("Cancel the raid check.");
                            }).clicked() {
                                self.state = QuestionState::None;
                            }
                        });
                    });
            },

            QuestionState::AskSavedBosses => {

                self.raid_id = if self.raid_id == -1 {
                    expansion_config.latest_expansion.as_ref().unwrap().raids.iter().last().unwrap().id
                } else {
                    self.raid_id
                };
                
                egui::Window::new("Saved bosses picker")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        
                        egui::ComboBox::from_label("Selected raid")
                            .selected_text(format!("{}", expansion_config.latest_expansion.as_ref().unwrap().raids.get(self.raid_id as usize).unwrap().identifier))
                            .show_ui(ui, |ui| {
                                for raid in expansion_config.latest_expansion.as_ref().unwrap().raids.iter() {
                                    ui.selectable_value(&mut self.raid_id, raid.id, raid.identifier.clone());
                                }
                            });

                        if self.prev_raid_id != self.raid_id {
                            self.saved_bosses.clear();
                            self.prev_raid_id = self.raid_id;
                        }

                        egui::ComboBox::from_label("Selected difficulty")
                            .selected_text(format!("{}", expansion_config.latest_expansion.as_ref().unwrap().raids.get(self.raid_id as usize).unwrap().difficulty.get(self.difficulty_id as usize).unwrap().difficulty_name))
                            .show_ui(ui, |ui| {
                                for difficulty in expansion_config.latest_expansion.as_ref().unwrap().raids.get(self.raid_id as usize).unwrap().difficulty.iter() {
                                    ui.selectable_value(&mut self.difficulty_id, difficulty.id, difficulty.difficulty_name.clone());
                                }
                            });

                        ui.horizontal(|ui| {
                            if ui.button("Enable all bosses").on_hover_ui(|ui| {
                                ui.label("Enable all bosses for this raid.");
                            }).clicked() {
                                self.saved_bosses.clear();
                                for i in 0..expansion_config.latest_expansion.as_ref().unwrap().raids.iter().find(|x| x.id == self.raid_id).unwrap().boss_names.len() {
                                    self.saved_bosses.push(i as i32);
                                }
                            };
    
                            if ui.button("Disable all bosses").on_hover_ui(|ui| {
                                ui.label("Disable all bosses for this raid.");
                            }).clicked() {
                                self.saved_bosses.clear();
                            };
                        });
                        
                        let mut bid = 0;
                        for boss in expansion_config.latest_expansion.as_ref().unwrap().raids.iter().find(|x| x.id == self.raid_id).unwrap().boss_names.iter() {
                            let mut tmp = self.saved_bosses.contains(&bid);
                            if ui.checkbox(&mut tmp, boss).changed() {
                                if tmp {
                                    self.saved_bosses.push(bid);
                                } else {
                                    self.saved_bosses.retain(|&x| x != bid);
                                }
                            }
                            bid += 1;
                        }
                        ui.label("");
                        ui.checkbox(&mut self.previous_difficulty, "Check if saved to the previous difficulty").on_hover_text("Check if the character has killed the selected bosses on the previous difficulty this reset. Does NOT count LFR.");
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
                    send_it = Some((self.raid_helper_url.clone(), self.ahead_of_the_curve, self.raid_id, self.difficulty_id, self.saved_bosses.clone(), self.previous_difficulty, self.player_only));
                    self.saved_bosses.clear();
                    self.raid_helper_url_error = false;
                    self.state = QuestionState::None;
                }
                else {
                    egui::Window::new("Raid Helper Checker")
                        .collapsible(false)
                        .resizable(false)
                        .show(ctx, |ui| {
                        if self.player_only == true {
                            ui.label("Please input the character name you would like to check.");
                        }
                        else {
                            ui.label("Please input the raid-helper URL that contains the signed characters you want to check.");
                        }

                        ui.text_edit_singleline(&mut self.raid_helper_url);

                        if self.raid_helper_url_error == true {
                            ui.label(egui::RichText::new("Invalid Raid Helper URL!").color(egui::Color32::RED));
                        }

                        ui.horizontal(|ui| {
                            if ui.button("Confirm").on_hover_ui(|ui| {
                                ui.label("Sign ups on this URL will be checked.");
                            }).clicked() {
                                if self.player_only == true {
                                    send_it = Some((self.raid_helper_url.clone(), self.ahead_of_the_curve, self.raid_id, self.difficulty_id, self.saved_bosses.clone(), self.previous_difficulty, self.player_only));
                                    self.saved_bosses.clear();
                                    self.raid_helper_url_error = false;
                                    self.state = QuestionState::None;
                                }
                                else {
                                    if let Some(url) = check_raidhelper_url(self.raid_helper_url.clone()) {
                                        self.raid_helper_url = url;
                                        
                                        send_it = Some((self.raid_helper_url.clone(), self.ahead_of_the_curve, self.raid_id, self.difficulty_id, self.saved_bosses.clone(), self.previous_difficulty, self.player_only));
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