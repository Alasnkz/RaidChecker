use egui::{CentralPanel, Hyperlink, SidePanel, Ui};

use crate::{checker::{armory_checker::AOTCStatus, check_player::PlayerData}, config::{self, settings::{PriorityChecks}}};

pub struct SignUpsUI {
    pub target_player: Option<PlayerData>
}

impl Default for SignUpsUI {
    fn default() -> Self {
        Self {
            target_player: None
        }
    }
}

impl SignUpsUI {
    pub fn draw_signups(&mut self, ctx: &eframe::egui::Context, settings: &mut config::settings::Settings, primary_people: &Vec<PlayerData>, 
        queued_people: &Vec<PlayerData>, should_recheck: &mut bool, clear_target: &mut bool, checked_player: &mut Option<PlayerData>) -> bool {
        
        if *clear_target {
            self.target_player = None;
            *clear_target = false;
        }
        
        SidePanel::left("side_panel")
        .width_range(200.0..=300.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if ui.button("Recheck").on_hover_text("Rechecks the sign-ups.").clicked() {
                    *should_recheck = true;
                }
                
                for player in primary_people.iter() {
                    // Decide what colour they should be?
                    if ui.label(egui::RichText::new(player.name.clone()).color(Self::colour_player_label(settings, player))).clicked() {
                        self.target_player = Some(player.clone());
                    }
                }
    
                ui.label("");
                ui.heading("Queued People");
                for player in queued_people.iter() {
                    // Decide what colour they should be?
                    if ui.label(egui::RichText::new(player.name.clone()).color(Self::colour_player_label(settings, player))).clicked() {
                        self.target_player = Some(player.clone());
                    }
                }
                false
            });
            false
        });

        CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.draw_player_info(ui, settings, None);
            }); 
        });

        if checked_player.is_some() {
            egui::Window::new("Player check")
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        self.draw_player_info(ui, settings, checked_player.clone());
                    });
                    if ui.button("Close").clicked() {
                        *checked_player = None;
                    } 
                });
        }
       
        false
    }

    pub fn colour_player_label(settings: &mut config::settings::Settings, player: &PlayerData) -> egui::Color32 {
        // Check ilvl
        if player.skip_reason.is_some() {
            let skip_colour = settings.skip_colour.unwrap();
            return egui::Color32::from_rgb(skip_colour[0], skip_colour[1], skip_colour[2]);
        }

        for prio in settings.check_priority.iter() {
            match prio {
                PriorityChecks::SavedKills => {
                    if player.saved_bosses.len() > 0 {
                        let saved_colour = settings.saved_colour.unwrap();
                        return egui::Color32::from_rgb(saved_colour[0], saved_colour[1], saved_colour[2]);
                    }
                },

                PriorityChecks::Ilvl => {
                    if player.ilvl < settings.average_ilvl {
                        let ilvl_colour = settings.ilvl_colour.unwrap();
                        return egui::Color32::from_rgb(ilvl_colour[0], ilvl_colour[1], ilvl_colour[2]);
                    }
                },

                PriorityChecks::Unkilled => {
                    if player.unkilled_bosses.len() > 0 {
                        let unkilled_colour = settings.unkilled_colour.unwrap();
                        return egui::Color32::from_rgb(unkilled_colour[0], unkilled_colour[1], unkilled_colour[2]);
                    }
                },

                PriorityChecks::Enchantments => {
                    if player.bad_gear.len() > 0 {
                        let bad_gear_colour = settings.bad_gear_colour.unwrap();
                        return egui::Color32::from_rgb(bad_gear_colour[0], bad_gear_colour[1], bad_gear_colour[2]);
                    }
                },

                PriorityChecks::RaidBuff => {
                    if player.buff_status > 0 {
                        let buff_colour = settings.buff_colour.unwrap();
                        return egui::Color32::from_rgb(buff_colour[0], buff_colour[1], buff_colour[2]);
                    }
                }
            }
        }

        egui::Color32::GREEN
    }

    pub fn draw_player_info(&mut self, ui: &mut Ui, settings: &mut config::settings::Settings, checked_player: Option<PlayerData>) {

        let player = if checked_player.is_some() {
            checked_player.unwrap()
        } else {
            if self.target_player.is_some() {
                self.target_player.clone().unwrap()
            } else {
                return;
            }
        };

        if player.skip_reason.is_some() {
            ui.label(format!("Skipped processing {}: {}", player.name.clone(), player.skip_reason.unwrap()));
            return;
        }

        ui.horizontal(|ui| {
            ui.add(Hyperlink::from_label_and_url("Armory", format!("{}", player.armory_url)));
            let converted_url = player.armory_url.clone().replace("worldofwarcraft.blizzard.com/en-gb", "www.warcraftlogs.com");
            ui.add(Hyperlink::from_label_and_url("Logs", format!("{}", converted_url)));
        });

        

        if player.ilvl < settings.average_ilvl {
            ui.label(format!("{} has an ilvl of {} which is below the average ilvl of {}", player.name.clone(), player.ilvl, settings.average_ilvl));

            ui.label("");
            ui.label("");
        }

        

        if player.unkilled_bosses.len() > 0 {
            ui.label(format!("{} has not killed the following bosses:", player.name.clone()));
            for boss in player.unkilled_bosses.iter() {
                ui.label(format!("\t{}", boss));
            }

            ui.label("");
            ui.label("");
        }

        if player.saved_bosses.len() > 0 {
            ui.label(format!("{} is saved to these bosses this reset:", player.name.clone()));
            for boss in player.saved_bosses.iter() {
                ui.label(format!("\t{}", boss));
            }

            ui.label("");
            ui.label("");
        }

        if player.bad_gear.len() > 0 {
            ui.label(format!("{} has gear that does not reach the requirements:", player.name.clone()));
            for gear in player.bad_gear.iter() {
                ui.label(format!("\t{}", gear));
            }
            ui.label("");
            ui.label("");
        }

        if player.buff_status > 0 {

            ui.label(egui::RichText::new(format!("{} is missing {}% raid buff!", player.name.clone(), player.buff_status * 3)).color(egui::Color32::from_rgb(255, 255, 0)));

            if player.buff_possible == false {
                ui.label(egui::RichText::new(format!("{} can not catch up with the raid buff this week, assuming they have not done any renown this week and they have 5000 renown catchup possible.", player.name.clone())).color(egui::Color32::from_rgb(255, 0, 0)));
            } else {
                ui.label(egui::RichText::new(format!("Assuming {} has not done any rep this week (catchup of 5000 renown). It is possible they can catch up and get a 3% damage/healing buff.", player.name.clone())).color(egui::Color32::from_rgb(0, 255, 0)));
                if player.buff_status > 1 {
                    ui.label(egui::RichText::new(format!("However, they will not be able to catch up to the other {}% damage/healing buffs they are missing.", (player.buff_status - 1) * 3)).color(egui::Color32::from_rgb(255, 0, 0)));
                }
            }
            ui.label("");
            ui.label("");
        }

        if player.aotc_status != AOTCStatus::None {
            if player.aotc_status == AOTCStatus::Error {
                ui.label(format!("{} has an error checking for AOTC.", player.name.clone()));
                return;
            } else {
                ui.label(format!("{} has AOTC on {}", player.name.clone(), match player.aotc_status {
                    AOTCStatus::Account => {
                        "their account, but not this character."
                    },

                    AOTCStatus::Character => {
                        "this character"
                    },

                    _ => { "Unknown" }
                }));
            }
        } else {
            ui.label("Player does not have AOTC.");
        }

        ui.label("");
        ui.label("");
        
        if player.discord_id.len() != 0 {
            ui.label(format!("Discord Mention: <@{}>", player.discord_id));
        }
    }
}