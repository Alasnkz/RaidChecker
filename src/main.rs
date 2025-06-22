#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use checker::{check_player::PlayerData, raid_questions::{QuestionState, RaidCheckQuestions}, raid_sheet::RaidSheet};
use chrono::{DateTime, TimeZone, Utc};
use config::{expansion_config::ExpansionsConfig, settings::Settings};
use egui::{TopBottomPanel, Window};
pub mod config;
pub mod checker;
use signups_ui::SignUpsUI;
pub mod signups_ui;
pub mod expansion_update;
pub mod settings_ui;
use config::last_raid::LastRaid;

use crate::{config::expansion_config::{ExpansionSeasons, Expansions}, expansion_update::ExpansionUpdateChecker};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native("Raid Helper Checker", options, Box::new(|_| Ok(Box::<RaidHelperCheckerApp>::default())))
}

struct RaidHelperCheckerApp {
    settings_ui: settings_ui::SettingsUi,
    draw_settings: bool,
    expansion_update_checker: expansion_update::ExpansionUpdateChecker,

    settings: config::settings::Settings,
    expansions: config::expansion_config::ExpansionsConfig,
    raid_questions: RaidCheckQuestions,
    raid_sheet: RaidSheet,
    signup_ui: SignUpsUI,
    last_raid: LastRaid,
    realms: config::realms::RealmJson,
    clear_target: bool,
    checked_player: Option<PlayerData>,
    draw_player_check: bool,

    win_title: String,
    win_title_change: bool,

    ask_json_update: bool,
    ask_update: bool,
}

impl Default for RaidHelperCheckerApp {
    fn default() -> Self {
        let mut app = Self {
            draw_settings: false,
            expansion_update_checker: ExpansionUpdateChecker::new(),
            settings_ui: settings_ui::SettingsUi::new(),
            settings: Settings::read_or_create("config.json").unwrap(),
            expansions: ExpansionsConfig::read_or_create("expansions.json").unwrap(),
            raid_questions: RaidCheckQuestions::default(),
            raid_sheet: RaidSheet::default(),
            signup_ui: SignUpsUI::default(),
            last_raid: LastRaid::read_or_create("last_raid.json").unwrap(),
            realms: config::realms::RealmJson::new(),
            clear_target: false,
            checked_player: None,
            draw_player_check: false,
            win_title: "Raid Helper Checker".to_string(),
            win_title_change: false,
            ask_json_update: false,
            ask_update: false,
        };
        app.reload_data();
        app.raid_sheet.init_from_last_raid(&app.last_raid);

        if ExpansionUpdateChecker::need_app_update() {
            app.ask_update = true;
        }

        if app.expansion_update_checker.need_expansion_json_update() {
            app.ask_json_update = true;
        }

        app
    }
}

impl RaidHelperCheckerApp{
    pub fn reload_data(&mut self) {
        self.expansions = ExpansionsConfig::read_or_create("expansions.json").unwrap();
        self.expansions.latest_expansion = Some(self.expansions.expansions.iter().find(|x| x.identifier == self.expansions.latest_expansion_identifier).unwrap_or(&Expansions::default()).clone());

        let mut season_ts_start = 0;
        let mut season_id = String::new();
        for season in self.expansions.latest_expansion.as_ref().unwrap().seasons.iter() {
            if season.season_start >= season_ts_start {
                if season.season_start != 0 {
                    let season_start: DateTime<Utc> = Utc.timestamp_opt(season.season_start, 0).unwrap();
                    let now: DateTime<Utc> = Utc::now();
                    if season_start <= now {
                        season_id = season.seasonal_identifier.clone();
                        season_ts_start = season.season_start;
                    } else {
                        println!("{} {} has not started yet, ignoring. Will activate on {}", self.expansions.latest_expansion_identifier, season.seasonal_identifier, season_start.format("%A, %B %d %Y").to_string());
                    }
                } else {
                    season_id = season.seasonal_identifier.clone();
                    season_ts_start = season.season_start;
                }
            }
        }
        self.expansions.latest_expansion.as_mut().unwrap().latest_season = self.expansions.latest_expansion.as_ref().unwrap().seasons.iter().find(|x| x.seasonal_identifier == season_id).cloned();
        self.win_title = format!("Raid Helper Checker ({} {})", self.expansions.latest_expansion.as_ref().unwrap().identifier, self.expansions.latest_expansion.as_ref().unwrap().latest_season.as_ref().unwrap_or(&ExpansionSeasons::default()).seasonal_identifier);
        self.win_title_change = true;
        self.settings.raid_id = if self.settings.raid_id == -1 {
            self.expansions.latest_expansion.as_ref().unwrap().latest_season.as_ref().unwrap().raids.last().unwrap().id
        } else {
            self.settings.raid_id
        };
    }
}

impl eframe::App for RaidHelperCheckerApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        if self.win_title_change {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.win_title.clone()));
            self.win_title_change = false;
        }

        if self.ask_update {
            Window::new("Update available")
                .show(ctx, |ui| {
                    ui.label("An update to raid helper checker is available. Clicking Download will bring you to the latest release in your browser.");
                    ui.horizontal(|ui| {
                        if ui.button("Download").clicked() {
                            ui.output_mut(|o| o.open_url = Some(egui::output::OpenUrl {
                                url: "https://github.com/Alasnkz/RaidChecker/releases/latest".to_string(),
                                new_tab: true,
                            }));
    
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        } else if ui.button("Cancel").clicked() {
                            self.ask_update = false;
                        }
                    });
                    
                });
        }
        else if self.ask_json_update{
            Window::new("Update available")
                .show(ctx, |ui| {
                    ui.label("An update to the expansion data is available.");
                    ui.horizontal(|ui| {
                        if ui.button("Download").clicked() {
                            if let Err(e) = self.expansion_update_checker.download_expansions_json() {
                                eprintln!("Failed to download expansions.json: {}", e);
                            } else {
                                self.reload_data();
                                self.ask_json_update = false;
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.ask_json_update = false;
                        }
                    });
                });
        } 
        else {
            TopBottomPanel::top("top_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    
                    if ui.button("Check sign-up URL").clicked() {
                        self.raid_questions.state = checker::raid_questions::QuestionState::AskSaved;
                        self.raid_questions.ignore_url_question = false;
                        self.raid_questions.player_only = false;
                    }

                    if ui.button("Check single character").clicked() {
                        self.draw_player_check = !self.draw_player_check;
                    }
                    if ui.button("Settings").clicked() {
                        self.draw_settings = !self.draw_settings;
                    }
                    
                });
            });

            let mut should_recheck = false;
            self.signup_ui.draw_signups(ctx, &mut self.settings, &self.raid_sheet.active_players, &self.raid_sheet.queued_players, &mut should_recheck, &mut self.clear_target, &mut self.checked_player);

            if should_recheck {
                self.raid_questions.state = QuestionState::AskSaved;
                let _ = self.raid_questions.ask_questions(ctx,  &self.expansions, Some(self.last_raid.raid_url.clone()), Some(false));
            }

            if self.raid_questions.state != checker::raid_questions::QuestionState::None {
                let ret = self.raid_questions.ask_questions(ctx, &self.expansions, None, None);
                if ret.is_some() {
                    let (url, raid_id, raid_difficulty, boss_kills, check_saved_prev_difficulty, player_only) = ret.unwrap();
                    self.raid_sheet.init(url, player_only, self.settings.clone(), self.expansions.clone(), self.realms.clone(), raid_id, raid_difficulty, boss_kills, check_saved_prev_difficulty, self.last_raid.clone());
                }
            }

            if self.raid_sheet.state != checker::raid_sheet::RaidSheetState::None {
                self.raid_sheet.draw(ctx, &mut self.last_raid, &mut self.clear_target, &mut self.checked_player);
            }

            if self.draw_settings == true {
                let ret = self.settings_ui.render(ctx, &mut self.settings, &mut self.expansions);
                self.draw_settings = !ret;
            }

            if self.draw_player_check == true {
                self.raid_questions.state = QuestionState::AskSaved;
                let _ = self.raid_questions.ask_questions(ctx, &self.expansions, None, Some(true));
                self.draw_player_check = false;
            }
        }
    }
}
