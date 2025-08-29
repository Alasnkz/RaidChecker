use egui::{CentralPanel, Hyperlink, Label, RichText, SidePanel, Ui};
use tracing::info;
use tracing_subscriber::fmt::format;

use crate::{checker::{armory_checker::AOTCStatus, check_player::PlayerData, raid_sheet::Player}, config::{self, settings::PriorityChecks}};

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
        queued_people: &Vec<PlayerData>, should_recheck: &mut bool, clear_target: &mut bool, checked_player: &mut Option<PlayerData>) -> Option<PlayerData> {
        
        let mut recheck_player = None;
        if *clear_target {
            self.target_player = None;
            *clear_target = false;
        }
        
        SidePanel::left("side_panel")
        .width_range(200.0..=300.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Recheck").on_hover_text("Rechecks the sign-ups.").clicked() {
                        *should_recheck = true;
                    }
                    
                    if ui.button("Summary").on_hover_text("Summarises the sign-ups.").clicked() {
                        self.target_player = None;
                    }
                });      

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
            });
        });

        CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.target_player.is_none() {
                    self.draw_summary(ui, settings, primary_people, queued_people);
                } else {
                    if self.draw_player_info(ui, settings, None) == true {
                        recheck_player = Some(self.target_player.clone().unwrap());
                    }
                }
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
       
        recheck_player
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
                    if player.bad_gear.len() > 0 || (player.num_embelishments != -1 && player.num_embelishments < settings.embelishments) {
                        let bad_gear_colour = settings.bad_gear_colour.unwrap();
                        return egui::Color32::from_rgb(bad_gear_colour[0], bad_gear_colour[1], bad_gear_colour[2]);
                    }
                },

                PriorityChecks::BadSocket => {
                    if player.bad_socket.len() > 0 {
                        let bad_socket_colour = settings.bad_socket_colour.unwrap();
                        return egui::Color32::from_rgb(bad_socket_colour[0], bad_socket_colour[1], bad_socket_colour[2]);
                    }
                }

                PriorityChecks::SpecialItem => {
                    if player.bad_special_item.len() > 0 {
                        let bad_gear_colour = settings.bad_special_item_colour.unwrap();
                        return egui::Color32::from_rgb(bad_gear_colour[0], bad_gear_colour[1], bad_gear_colour[2]);
                    }
                },

                PriorityChecks::RaidBuff => {
                    if player.buff_status.iter().any(|x| x.1.1 > 0) {
                        let buff_colour: [u8; 4] = settings.buff_colour.unwrap();
                        return egui::Color32::from_rgb(buff_colour[0], buff_colour[1], buff_colour[2]);
                    }
                },

                PriorityChecks::MissingTier => {
                    if player.tier_count != -1 && player.tier_count < 4 {
                        let tier_colour: [u8; 4] = settings.missing_tier_colour.unwrap();
                        return egui::Color32::from_rgb(tier_colour[0], tier_colour[1], tier_colour[2]);
                    }
                }
            }
        }

        egui::Color32::GREEN
    }

    pub fn draw_summary(&mut self, ui: &mut Ui, settings: &mut config::settings::Settings, primary_people: &Vec<PlayerData>, queued_people: &Vec<PlayerData>) {
        let mut bad = 0;

        let mut bad_primary = Vec::new();
        let mut bad_secondary = Vec::new();

        let combined = primary_people.iter().chain(queued_people.iter()).collect::<Vec<&PlayerData>>();
        if combined.len() == 0 {
            ui.label("A general summary of the sign-ups will be shown here.");
            return;
        }

        for player in combined.iter() {
            let mut set_bad = false;
            let mut bad_message = format!("<@{}> Your signed character {} does not meet the requirements:\n", player.discord_id, player.name.clone());
            if player.skip_reason.is_some() {
                bad += 1;
                continue;
            }

            if player.ilvl < settings.average_ilvl {
                set_bad = true;
                bad_message += format!("Your ilvl {} does not match the required ilvl for this raid: {}\n", player.ilvl, settings.average_ilvl).as_str();
            }

            let mut raid_name = String::new();
            if player.unkilled_bosses.len() > 0 {
                
                set_bad = true;
                for boss in player.unkilled_bosses.iter() {
                    if raid_name != boss.0 {
                        raid_name = boss.0.clone();
                        bad_message += format!("You have not killed the following bosses in {}:\n", raid_name).as_str();
                    }
                    bad_message += format!("\t{}\n", boss.1).as_str();
                }
            }

            raid_name = String::new();
            if player.saved_bosses.len() > 0 {
                set_bad = true;
                for boss in player.saved_bosses.iter() {
                    if raid_name != boss.0 {
                        raid_name = boss.0.clone();
                        bad_message += format!("You are saved to the following bosses in {}:\n", raid_name).as_str();
                    }
                    bad_message += format!("\t{}\n", boss.1).as_str();
                }
            }

            if player.bad_gear.len() > 0 {
                set_bad = true;
                bad_message += format!("You have the following gear that does not meet the requirements:\n").as_str();
                for item in player.bad_gear.iter() {
                    bad_message += format!("\t{}\n", item).as_str();
                }
            }

            if player.bad_socket.len() > 0 {
                set_bad = true;
                bad_message += format!("You have the following items that do not meet the socket requirements:\n").as_str();
                for item in player.bad_socket.iter() {
                    bad_message += format!("\t{}\n", item).as_str();
                }
            }

            if player.bad_special_item.len() > 0 {
                set_bad = true;
                bad_message += format!("You have the following special items that do not meet the requirements:\n").as_str();
                for item in player.bad_special_item.iter() {
                    bad_message += format!("\t{}\n", item).as_str();
                }
            }

            if player.num_embelishments != -1 && player.num_embelishments < settings.embelishments {
                set_bad = true;
                bad_message += format!("You are missing **{}** embelishments, you need at least **{}**\n", settings.embelishments - player.num_embelishments, settings.embelishments).as_str();
            }

            for (_, (raid_name, missing_buff_count, missing_buff_possible, missing_buff_size, missing_buff_catchup)) in player.buff_status.iter() {
                if *missing_buff_count > 0 {
                    set_bad = true;
                    bad_message += format!("You are missing **{}%** raid buff from {raid_name}!\n", missing_buff_count * missing_buff_size).as_str();
                    
                    if *missing_buff_possible == false {
                        bad_message += format!("You can **not** catch up with {raid_name}'s raid buff this week.\n").as_str();
                    } else {
                        bad_message += format!("You can get a **{}%** raid buff this week, **assuming you have not done any renown this week**. ({} catchup)\n", missing_buff_size, missing_buff_catchup).as_str();
                        if *missing_buff_count > 1 {
                            bad_message += format!("Due to catch up being capped, **you will miss {}%** of the raid buff.\n", (missing_buff_count - 1) * missing_buff_size).as_str();
                        }
                    }
                }
            }

            

            if set_bad == true{
                if player.queued {
                    bad_secondary.push(bad_message);
                } else {
                    bad_primary.push(bad_message);
                }
                bad += 1;
            }
        }

        ui.label(format!("{}/{} haved passed the checks.", (primary_people.len() + queued_people.len()) - bad, primary_people.len() + queued_people.len()));
        ui.label("");

        for message in bad_primary.iter() {
            ui.label(message.clone());
            ui.label("");
            ui.label("");
        }

        ui.heading("Queued People Issues");
        for message in bad_secondary.iter() {
            ui.label(message.clone());
            ui.label("");
            ui.label("");
        }
    }

    pub fn draw_player_info(&mut self, ui: &mut Ui, settings: &mut config::settings::Settings, checked_player: Option<PlayerData>) -> bool {

        let mut should_recheck = false;
        let player = if checked_player.is_some() {
            checked_player.clone().unwrap()
        } else {
            if self.target_player.is_some() {
                self.target_player.clone().unwrap()
            } else {
                return false;
            }
        };

        if player.skip_reason.is_some() {
            ui.label(format!("Skipped processing {}: {}", player.name.clone(), player.skip_reason.unwrap()));
            return false;
        }

        ui.horizontal(|ui| {
            ui.add(Hyperlink::from_label_and_url("Armory", format!("{}", player.armory_url)));
            let converted_url = player.armory_url.clone().replace("worldofwarcraft.blizzard.com/en-gb", "www.warcraftlogs.com");
            ui.add(Hyperlink::from_label_and_url("Logs", format!("{}", converted_url)));
            if checked_player.as_ref().is_none() && ui.button("Recheck").on_hover_text("Rechecks this player.").clicked() == true {
                should_recheck = true;
            }
        });

        

        if player.ilvl < settings.average_ilvl {
            ui.label(format!("{} has an ilvl of {} which is below the average ilvl of {}", player.name.clone(), player.ilvl, settings.average_ilvl));

            ui.label("");
            ui.label("");
        }

        let mut raid_name = String::new();
        if player.unkilled_bosses.len() > 0 {
            ui.label(format!("{} has not killed the following bosses:", player.name.clone()));
            for boss in player.unkilled_bosses.iter() {
                if raid_name != boss.0 {
                    raid_name = boss.0.clone();
                    ui.heading(format!("{}", raid_name));
                }

                ui.label(format!("\t{}", boss.1));
            }

            ui.label("");
            ui.label("");
        }

        let mut raid_name = String::new();
        if player.saved_bosses.len() > 0 {
            ui.label(format!("{} is saved to these bosses this reset:", player.name.clone()));
            for boss in player.saved_bosses.iter() {
                if raid_name != boss.0 {
                    raid_name = boss.0.clone();
                    ui.heading(format!("{}", raid_name));
                }

                ui.label(format!("\t{}", boss.1));
            }

            ui.label("");
            ui.label("");
        }

        let gear_issue = player.bad_gear.len() > 0 || player.bad_socket.len() > 0 || player.bad_special_item.len() > 0 || (player.num_embelishments != -1 && player.num_embelishments < settings.embelishments);
        if gear_issue {
            ui.label(format!("{} has gear that does not meet the requirements:", player.name.clone()));
        }

        if player.bad_gear.len() > 0 {
            for gear in player.bad_gear.iter() {
                ui.label(format!("\t{}", gear));
            }
        }

        if player.bad_special_item.len() > 0 {
            let special_item_colour = settings.bad_special_item_colour.unwrap();
            for gear in player.bad_special_item.iter() {
                ui.label(egui::RichText::new(format!("\t{}", gear)).color(egui::Color32::from_rgb(special_item_colour[0], special_item_colour[1], special_item_colour[2])));
            }
        }

        if player.bad_socket.len() > 0 {
            for gear in player.bad_socket.iter() {
                ui.label(format!("\t{}", gear));
            }
        }

        if player.num_embelishments != -1 && player.num_embelishments < settings.embelishments {
            ui.label(egui::RichText::new(format!("{} is missing {} embelishments", player.name.clone(), settings.embelishments - player.num_embelishments)).color(egui::Color32::from_rgb(255, 0, 0)));
        }
        
        if gear_issue {
            ui.label("");
            ui.label("");
        }
        
        for (_, (raid_name, missing_buff_count, missing_buff_possible, missing_buff_size, missing_buff_catchup)) in player.buff_status.iter() {
            if *missing_buff_count > 0 {
                ui.label(egui::RichText::new(format!("{} is missing {}% raid buff for {raid_name}!", player.name.clone(), missing_buff_count * missing_buff_size)).color(egui::Color32::from_rgb(255, 255, 0)));

                if *missing_buff_possible == false {
                    ui.label(egui::RichText::new(format!("{} can not catch up with {raid_name}'s raid buff this week, assuming they have not done any renown this week and they have {} renown catchup possible.", player.name.clone(), missing_buff_catchup)).color(egui::Color32::from_rgb(255, 0, 0)));
                } else {
                    ui.label(egui::RichText::new(format!("Assuming {} has not done any rep this week (catchup of {} renown). It is possible they can catch up and get a {}% damage/healing buff.", player.name.clone(), missing_buff_catchup, missing_buff_size)).color(egui::Color32::from_rgb(0, 255, 0)));
                    if *missing_buff_count > 1 {
                        ui.label(egui::RichText::new(format!("However, they will not be able to catch up to the other {}% damage/healing buffs they are missing.", (missing_buff_count - 1) * missing_buff_size)).color(egui::Color32::from_rgb(255, 0, 0)));
                    }
                }
                ui.label("");
                ui.label("");
            }
        }

        if player.tier_count != -1{
            ui.label(format!("{} has {} tier pieces.", player.name.clone(), player.tier_count));
        }

        for (_, (raid_name, aotc_status)) in player.aotc_status.iter() {
            let mut string = String::new();
            match aotc_status {
                AOTCStatus::Account => {
                    string = format!("{} has {raid_name} AOTC on their account, but not on this character.", player.name.clone());
                },

                AOTCStatus::Character => {
                    string = format!("{} has {raid_name} AOTC on this character.", player.name.clone());
                },

                AOTCStatus::CuttingEdge(account, character, heroic_kill) => {
                    if *account == true && *character == false {
                        if *heroic_kill == true {
                            string = format!("{} has {raid_name} Cutting Edge on their account, but on this character, they have only earned AOTC.", player.name.clone());
                        } else {
                            string = format!("{} has {raid_name} Cutting Edge on their account, but not on this character. This character has not earned AOTC.", player.name.clone());
                        }
                    } else if *account == true && *character == true {
                        if *heroic_kill == false {
                            string = format!("{} has {raid_name} Cutting Edge on this character, but has not earned AOTC on this character.", player.name.clone());
                        } else {
                            string = format!("{} has {raid_name} Cutting Edge on this character.", player.name.clone());
                        }
                        
                    }
                },

                AOTCStatus::None => {
                    string = format!("{} does not have {raid_name} AOTC.", player.name.clone());
                },

                AOTCStatus::Skipped => {
                    string = format!("");
                },

                _ => { 
                    string = format!("Unknown {raid_name} AOTC status.");
                }
            }

            if string.len() > 0 {
                ui.label(string);
            }
            
        }

        ui.label("");
        ui.label("");
        
        if player.discord_id.len() != 0 {
            ui.label(format!("Discord Mention: <@{}>", player.discord_id));
        }

        should_recheck
    }
}