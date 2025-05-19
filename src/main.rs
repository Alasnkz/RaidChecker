#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use checker::{check_player::PlayerData, raid_questions::{QuestionState, RaidCheckQuestions}, raid_sheet::RaidSheet};
use config::{expansion_config::ExpansionsConfig, settings::Settings};
use egui::TopBottomPanel;
pub mod config;
pub mod checker;
use signups_ui::SignUpsUI;
pub mod signups_ui;
pub mod expansion_update;
pub mod settings_ui;
use config::last_raid::LastRaid;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native("Raid Helper Checker", options, Box::new(|_| Ok(Box::<RaidHelperCheckerApp>::default())))
}

struct RaidHelperCheckerApp {
    version: i32,
    settings_ui: settings_ui::SettingsUi,
    draw_settings: bool,
    
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
}

impl Default for RaidHelperCheckerApp {
    fn default() -> Self {
        let mut app = Self {
            version: 1,
            draw_settings: false,
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
        };
        app.expansions.latest_expansion = Some(app.expansions.expansions.iter().find(|x| x.identifier == app.expansions.latest_expansion_identifier).unwrap().clone());
        
        app.settings.raid_id = if app.settings.raid_id == -1 {
            app.expansions.latest_expansion.as_ref().unwrap().raids.iter().last().unwrap().id
        } else {
            app.settings.raid_id
        };

        app.raid_sheet.init_from_last_raid(&app.last_raid);
        app
    }
}

impl eframe::App for RaidHelperCheckerApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
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
            let _ = self.raid_questions.ask_questions(ctx, &self.expansions, None, None);
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
