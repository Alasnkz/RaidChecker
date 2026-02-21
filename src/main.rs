#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use std::{fs::OpenOptions, io::{self, BufWriter}};

use checker::{check_player::PlayerData, raid_questions::{QuestionState, RaidCheckQuestions}, raid_sheet::RaidSheet};
use chrono::{DateTime, TimeZone, Utc};
use config::{expansion_config::ExpansionsConfig, settings::Settings};
use egui::{TopBottomPanel, Visuals, Window};
pub mod config;
pub mod checker;
use signups_ui::SignUpsUI;
pub mod signups_ui;
pub mod expansion_update;
pub mod settings_ui;
use config::last_raid::LastRaid;
use tracing::{error, info};
use tracing_subscriber::layer::Layer;
use tracing_subscriber::{fmt, layer::SubscriberExt, Registry};
use tracing_subscriber::EnvFilter;

use crate::{checker::{check_player::slug_to_name, raid_sheet::{Player, PlayerOnlyCheckType}}, config::expansion_config::{ExpansionSeasons, Expansion}, expansion_update::ExpansionUpdateChecker};

static SHOULD_RECHECK_ALL: u8 = 1;
static SHOULD_RECHECK_ATTENDANCE: u8 = 2;

fn init_logging() {
    let log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("RaidChecker.log")
        .expect("Unable to open log file");

    let file_writer = move || BufWriter::new(log_file.try_clone().expect("Failed to clone log file"));

    let file_filter = EnvFilter::try_new("info").unwrap();
    let console_filter: EnvFilter = EnvFilter::try_new("info").unwrap();

    let file_layer = fmt::layer()
        .with_writer(file_writer)
        .with_ansi(false)
        .with_line_number(true)
        .with_file(true)
        .with_filter(file_filter);

    let console_layer = fmt::layer()
        .with_ansi(true)
        .with_filter(console_filter);

    let subscriber = Registry::default()
        .with(file_layer)
        .with(console_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global subscriber");
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    init_logging();
    eframe::run_native("Raid Checker", options, Box::new(|_| Ok(Box::<RaidHelperCheckerApp>::default())))
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
            win_title: "Raid Checker".to_string(),
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
        let mut expansion_ts_start = 0;
        let mut expansion_identifier = String::new();
        for expansion in self.expansions.expansions.iter() {
            if expansion.expansion_start >= expansion_ts_start {
                if expansion.expansion_start != 0 {
                    let expansion_start: DateTime<Utc> = Utc.timestamp_opt(expansion.expansion_start, 0).unwrap();
                    let now: DateTime<Utc> = Utc::now();
                    if expansion_start <= now {
                        expansion_identifier = expansion.identifier.clone();
                        expansion_ts_start = expansion.expansion_start;
                    } else {
                        info!("{} has not started yet, ignoring. Will activate on {}", expansion.name, expansion_start.format("%A, %B %d %Y").to_string());
                    }
                } else if expansion.expansion_start >= 0 && expansion_ts_start == 0 {
                    expansion_identifier = expansion.identifier.clone();
                    expansion_ts_start = expansion.expansion_start;
                }
            }
        }
        self.expansions.latest_expansion = Some(self.expansions.expansions.iter().find(|x| x.identifier == expansion_identifier).unwrap_or(&Expansion::default()).clone());

        let mut season_ts_start = 0;
        let mut season_id = String::new();
        for season in self.expansions.latest_expansion.clone().unwrap().seasons.iter() {
            if season.season_start >= season_ts_start {
                if season.season_start != 0 {
                    let season_start: DateTime<Utc> = Utc.timestamp_opt(season.season_start, 0).unwrap();
                    let now: DateTime<Utc> = Utc::now();
                    if season_start <= now {
                        season_id = season.seasonal_identifier.clone();
                        season_ts_start = season.season_start;
                    } else {
                        info!("{} {} has not started yet, ignoring. Will activate on {}", self.expansions.latest_expansion.as_ref().unwrap().identifier, season.seasonal_identifier, season_start.format("%A, %B %d %Y").to_string());
                        self.expansions.latest_expansion.as_mut().unwrap().seasons.retain(|x| x.seasonal_identifier != season.seasonal_identifier);
                    }
                } else {
                    season_id = season.seasonal_identifier.clone();
                    season_ts_start = season.season_start;
                }
            }
        }
        self.expansions.latest_expansion.as_mut().unwrap().latest_season = self.expansions.latest_expansion.as_ref().unwrap().seasons.iter().find(|x| x.seasonal_identifier == season_id).cloned();
        self.win_title = format!("Raid Checker ({} {})", self.expansions.latest_expansion.as_ref().unwrap().name, self.expansions.latest_expansion.as_ref().unwrap().latest_season.as_ref().unwrap_or(&ExpansionSeasons::default()).seasonal_identifier);
        self.win_title_change = true;

        if self.expansions.latest_expansion.as_ref().unwrap().latest_season.is_none() {
            return;
        }

        for raid in self.expansions.latest_expansion.clone().unwrap().latest_season.unwrap().raids.iter() {
            if raid.release_time != 0 {
                let raid_launch: DateTime<Utc> = Utc.timestamp_opt(raid.release_time, 0).unwrap();
                let now: DateTime<Utc> = Utc::now();
                if raid_launch > now {
                    info!("{} raid {} ({}) has not launched yet, ignoring. Will activate on {}", self.expansions.latest_expansion.as_ref().unwrap().name, raid.identifier, self.expansions.latest_expansion.clone().unwrap().latest_season.unwrap().seasonal_identifier, raid_launch.format("%A, %B %d %Y").to_string());
                    self.expansions.latest_expansion.as_mut().unwrap().seasons.last_mut().unwrap().raids.retain(|x| x.identifier != raid.identifier);
                    self.expansions.latest_expansion.as_mut().unwrap().latest_season.as_mut().unwrap().raids.retain(|x| x.identifier != raid.identifier);
                }
            }
        }

        for item in self.expansions.latest_expansion.as_mut().unwrap().latest_season.clone().unwrap().seasonal_slot_data.iter_mut() {
            if item.release_time != 0 {
                let release_time: DateTime<Utc> = Utc.timestamp_opt(item.release_time, 0).unwrap();
                let now: DateTime<Utc> = Utc::now();
                if release_time > now {
                    info!("{} {} gear {} has not launched yet, ignoring. Will activate on {}", self.expansions.latest_expansion.as_ref().unwrap().name, self.expansions.latest_expansion.clone().unwrap().latest_season.unwrap().seasonal_identifier, item.slot, release_time.format("%A, %B %d %Y").to_string());
                    self.expansions.latest_expansion.as_mut().unwrap().seasons.last_mut().unwrap().seasonal_slot_data.retain(|x| x.slot != item.slot);
                    self.expansions.latest_expansion.as_mut().unwrap().latest_season.as_mut().unwrap().seasonal_slot_data.retain(|x| x.slot != item.slot);
                }
            }
        }

        if self.expansions.latest_expansion_identifier != expansion_identifier {
            info!("Resetting saved raids data, expansion has changed.");
            self.settings.required_raids.clear();
        }

        self.expansions.latest_expansion_identifier = expansion_identifier.clone();
    }
}

impl eframe::App for RaidHelperCheckerApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        if self.win_title_change {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.win_title.clone()));
            self.win_title_change = false;
            ctx.set_visuals(Visuals::dark());
        }

        if self.ask_update {
            Window::new("Update available")
                .show(ctx, |ui| {
                    ui.label("An update to Raid Checker is available. Clicking Download will bring you to the latest release in your browser.");
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
            Window::new("Expansion data update available")
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
                        self.raid_questions.player_only = PlayerOnlyCheckType::None;
                    }

                    if ui.button("Check single character").clicked() {
                        self.draw_player_check = !self.draw_player_check;
                    }
                    if ui.button("Settings").clicked() {
                        self.draw_settings = !self.draw_settings;
                    }

                });
            });

            let mut should_recheck: u8 = 0;
            let recheck_player = self.signup_ui.draw_signups(ctx, &mut self.settings, &self.raid_sheet.active_players, &self.raid_sheet.queued_players, &mut should_recheck, &mut self.clear_target, &mut self.checked_player);
            if recheck_player.is_some() {
                let armory_url = recheck_player.as_ref().unwrap().armory_url.clone();
                let parts: Vec<_> = armory_url.trim_end_matches('/').rsplitn(3, '/').collect();
                let realm = slug_to_name(parts[1], &self.realms);
                if realm.is_none() {
                    error!("Realm from slug not found for player: {} {}", recheck_player.as_ref().unwrap().name, parts[1]);
                    return;
                }
                info!("Rechecking player: {} {}-{}", recheck_player.as_ref().unwrap().name, parts[0], realm.as_ref().unwrap());
                self.raid_questions.state = QuestionState::AskSaved;
                self.raid_questions.ignore_url_question = true;
                self.raid_questions.player_only = PlayerOnlyCheckType::PlayerFromSheet(recheck_player.as_ref().unwrap().discord_id.clone());
                let _ = self.raid_questions.ask_questions(ctx, &self.expansions, Some(format!("{}-{}", parts[0], realm.unwrap())), Some(PlayerOnlyCheckType::PlayerFromSheet(recheck_player.as_ref().unwrap().discord_id.clone())));
            }

            if should_recheck == SHOULD_RECHECK_ALL {
                self.raid_questions.state = QuestionState::AskSaved;
                let _ = self.raid_questions.ask_questions(ctx,  &self.expansions, Some(self.last_raid.raid_url.clone()), Some(PlayerOnlyCheckType::None));
            } else if should_recheck == SHOULD_RECHECK_ATTENDANCE {
                egui::Window::new("Rechecking raid plan")
                    .show(ctx, |ui| {
                        ui.label("Rechecking raid plan attendance data...");
                    });

                self.raid_sheet.recheck_raid_plan(&mut self.last_raid);
            }

            if self.raid_questions.state != checker::raid_questions::QuestionState::None {
                let ret = self.raid_questions.ask_questions(ctx, &self.expansions, None, None);
                if ret.is_some() {
                    let (url, boss_kills, player_only) = ret.unwrap();
                    self.raid_sheet.init(url, player_only.clone(), self.settings.clone(), self.expansions.clone(), self.realms.clone(), boss_kills, self.last_raid.clone());

                    if player_only != PlayerOnlyCheckType::Player && player_only != PlayerOnlyCheckType::None {
                        self.raid_questions.raid_helper_url = String::new();
                    }
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
                self.raid_questions.ignore_url_question = false;
                self.raid_questions.player_only = PlayerOnlyCheckType::Player;
                let _ = self.raid_questions.ask_questions(ctx, &self.expansions, None, Some(PlayerOnlyCheckType::Player));
                self.draw_player_check = false;
            }
        }
    }
}
