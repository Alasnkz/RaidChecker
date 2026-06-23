use std::collections::BTreeMap;

use chrono::{DateTime, Local, TimeZone, Utc};
use egui::{CentralPanel, Hyperlink, Label, RichText, SidePanel, Ui, epaint::color};
use tracing::info;
use tracing_subscriber::fmt::format;

use crate::{SHOULD_RECHECK_ALL, SHOULD_RECHECK_ATTENDANCE, checker::{armory_checker::RaidProgressStatus, check_player::PlayerData, gear_checker::GearChecker, raid_sheet::{Player, RAID_PLAN_CANCELLED, RAID_PLAN_UNCONFIRMED, RaidSheetType}, saved_checker::SavedChecker}, config::{self, expansion_config::ExpansionsConfig, settings::PriorityChecks}};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct BossKey {
    raid_id: usize,
    boss_id: usize,
}

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
    pub fn draw_signups(&mut self, ctx: &eframe::egui::Context, settings: &mut config::settings::Settings, expansions: &ExpansionsConfig, primary_people: &mut Vec<PlayerData>, 
        queued_people: &mut Vec<PlayerData>, sheet_type: RaidSheetType, should_recheck: &mut u8, clear_target: &mut bool, checked_player: &mut Option<PlayerData>) -> Option<PlayerData> {
        
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
                        *should_recheck = SHOULD_RECHECK_ALL;
                    }
                    
                    if ui.button("Summary").on_hover_text("Summarises the sign-ups.").clicked() {
                        self.target_player = None;
                    }
                    
                    if ui.button("Raid Plan recheck").on_hover_text("Rechecks the raid plan to see if there's any new attendance confirmations.").clicked() {
                        *should_recheck = SHOULD_RECHECK_ATTENDANCE; 
                    }
                });      

                let roles = ["Tank", "Healer", "Melee", "Ranged", "DPS", "Skipped"];


                let mut primary_players = primary_people.clone();
                for role in roles.iter() {
                    ui.push_id(format!("{role}P"), |ui| {
                        if primary_people.iter_mut().find(|x| x.class_name == role.to_lowercase() || x.role_name == role.to_lowercase()).is_none() {
                            return;
                        }
                        egui::CollapsingHeader::new(*role)
                        .default_open(true)
                        .show(ui, |ui| {
                            for player in primary_people.iter_mut().filter(|x| x.class_name == role.to_lowercase() || x.role_name == role.to_lowercase()) {
                                let mut label_name = if sheet_type == RaidSheetType::Classes {
                                    format!("{} ({})", player.name.clone(), player.class_name.clone())
                                } else {
                                    player.name.clone()
                                };

                                if settings.current_preset.regulars.as_ref().unwrap_or(&BTreeMap::new()).get(&player.discord_id).is_some() {
                                    label_name = format!("⭐ {}", label_name);
                                }

                                if ui.label(egui::RichText::new(label_name).color(self.colour_player_label(settings, player, expansions))).clicked() {
                                    self.target_player = Some(player.clone());
                                }

                                primary_players.remove(primary_players.iter().position(|x| x.discord_id == player.discord_id).unwrap());
                            }
                        });
                    });
                }

                if primary_players.len() > 0 {
                    ui.heading(egui::RichText::new("Recheck needed for new headers!").color(egui::Color32::YELLOW));
                    ui.heading("Primary People");
                }
                for player in primary_players.iter_mut() {
                    if ui.label(egui::RichText::new(player.name.clone()).color(self.colour_player_label(settings, player, expansions))).clicked() {
                        self.target_player = Some(player.clone());
                    }
                }
                
                if queued_people.len() > 0 {
                    ui.label("");
                    ui.heading("Queued People");
                }

                let mut queued_players = queued_people.clone();
                for role in roles.iter() {
                    ui.push_id(format!("{role}S"), |ui| {
                        if queued_people.iter().find(|x| x.class_name == role.to_lowercase() || x.role_name == role.to_lowercase()).is_none() {
                            return;
                        }
                        
                        egui::CollapsingHeader::new(*role)
                        .default_open(true)
                        .show(ui, |ui| {
                            for player in queued_people.iter_mut().filter(|x| x.class_name == role.to_lowercase() || x.role_name == role.to_lowercase()) {
                                let mut label_name = if sheet_type == RaidSheetType::Classes {
                                    format!("{} ({})", player.name.clone(), player.class_name.clone())
                                } else {
                                    player.name.clone()
                                };

                                if settings.current_preset.regulars.as_ref().unwrap_or(&BTreeMap::new()).get(&player.discord_id).is_some() {
                                    label_name = format!("⭐ {}", label_name);
                                }

                                if ui.label(egui::RichText::new(label_name).color(self.colour_player_label(settings, player, expansions))).clicked() {
                                    self.target_player = Some(player.clone());
                                }

                                queued_players.remove(queued_players.iter().position(|x| x.discord_id == player.discord_id).unwrap());
                            }
                        });
                    });
                }

                if queued_players.len() > 0 {
                    ui.heading(egui::RichText::new("Recheck needed for new headers!").color(egui::Color32::YELLOW));
                }

                for player in queued_players.iter_mut() {
                    if ui.label(egui::RichText::new(player.name.clone()).color(self.colour_player_label(settings, player, expansions))).clicked() {
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
                    if self.draw_player_info(ui, settings, expansions, &mut None) == true {
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
                        self.draw_player_info(ui, settings, expansions, checked_player);
                    });
                    if ui.button("Close").clicked() {
                        *checked_player = None;
                    } 
                });
        }
       
        recheck_player
    }

    pub fn colour_player_label(&mut self, settings: &mut config::settings::Settings, player: &mut PlayerData, expansions: &ExpansionsConfig) -> egui::Color32 {
        // Check ilvl
        if player.skip_reason.is_some() {
            let skip_colour = settings.current_preset.skip_colour.unwrap();
            return egui::Color32::from_rgb(skip_colour[0], skip_colour[1], skip_colour[2]);
        }

        if settings.dirty_state != player.dirty_state {
            if player.character.gear.len() > 0 {
                let (bad_gear, bad_socket, bad_item, embelishments) = GearChecker::check_gear(&player.character, settings, expansions);
                let pvp_gear = GearChecker::check_pvp_gear(&player.character.gear, expansions);
                let tier_count = GearChecker::check_tier_pieces(&player.character.gear, expansions);
                player.bad_gear = bad_gear;
                player.bad_socket = bad_socket;
                player.bad_special_item = bad_item;
                player.num_embelishments = embelishments;
                player.pvp_gear = pvp_gear;
                player.tier_count = tier_count;
            }
            player.dirty_state = settings.dirty_state;
           
            if self.target_player.is_some() && self.target_player.as_ref().unwrap().discord_id == player.discord_id {
                self.target_player = Some(player.clone());
            }
        }

        for prio in settings.current_preset.check_priority.iter() {
            match prio {
                PriorityChecks::SavedKills => {
                    for raid in &player.raid_data {
                        if settings.current_preset.saved_raids.get(&(*raid.0 as i32)).is_some() {
                            for boss in &raid.1.bosses {
                                for difficulty in &boss.1.difficulties {
                                    let saved_difficulty = settings.current_preset.saved_raids.get(&(*raid.0 as i32)).unwrap().difficulty.get(&(*difficulty.0 as i32));
                                    if saved_difficulty.is_some() {
                                        if saved_difficulty.unwrap().boss_ids.get(boss.1.boss_id).is_some() {
                                            if difficulty.1.boss_kill_time.is_some() {
                                                if difficulty.1.boss_kill_time.unwrap() > SavedChecker::get_wednesday_reset_timestamp() as u64 {
                                                    let saved_colour = settings.current_preset.saved_colour.unwrap();
                                                    return egui::Color32::from_rgb(saved_colour[0], saved_colour[1], saved_colour[2]);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }                    
                },

                PriorityChecks::Ilvl => {
                    if player.ilvl < settings.current_preset.average_ilvl{
                        let ilvl_colour = settings.current_preset.ilvl_colour.unwrap();
                        return egui::Color32::from_rgb(ilvl_colour[0], ilvl_colour[1], ilvl_colour[2]);
                    }
                },

                PriorityChecks::Unkilled => {
                    for raid in &player.raid_data {
                        if settings.current_preset.required_raids.get(&(*raid.0 as i32)).is_some() {
                            for boss in &raid.1.bosses {
                                for difficulty in &boss.1.difficulties {
                                    let required_difficulties = settings.current_preset.required_raids.get(&(*raid.0 as i32)).unwrap().difficulty.get(&(*difficulty.0 as i32));
                                    if required_difficulties.is_some() {
                                        if required_difficulties.unwrap().boss_ids.get(boss.1.boss_id).is_some() {
                                            if difficulty.1.killed_before == false {
                                                let unkilled_colour = settings.current_preset.unkilled_colour.unwrap();
                                                return egui::Color32::from_rgb(unkilled_colour[0], unkilled_colour[1], unkilled_colour[2]);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },

                PriorityChecks::Enchantments => {
                    if player.bad_gear.len() > 0 || (player.num_embelishments != -1 && player.num_embelishments < settings.current_preset.embelishments) {
                        let bad_gear_colour = settings.current_preset.bad_gear_colour.unwrap();
                        return egui::Color32::from_rgb(bad_gear_colour[0], bad_gear_colour[1], bad_gear_colour[2]);
                    }
                },

                PriorityChecks::BadSocket => {
                    if player.bad_socket.len() > 0 {
                        let bad_socket_colour = settings.current_preset.bad_socket_colour.unwrap();
                        return egui::Color32::from_rgb(bad_socket_colour[0], bad_socket_colour[1], bad_socket_colour[2]);
                    }
                }

                PriorityChecks::SpecialItem => {
                    if player.bad_special_item.len() > 0 {
                        let bad_gear_colour = settings.current_preset.bad_special_item_colour.unwrap();
                        return egui::Color32::from_rgb(bad_gear_colour[0], bad_gear_colour[1], bad_gear_colour[2]);
                    }
                },

                PriorityChecks::RaidBuff => {
                    if player.buff_status.iter().any(|x| x.1.1 > 0) {
                        let buff_colour: [u8; 4] = settings.current_preset.buff_colour.unwrap();
                        return egui::Color32::from_rgb(buff_colour[0], buff_colour[1], buff_colour[2]);
                    }
                },

                PriorityChecks::MissingTier => {
                    if player.tier_count != -1 && player.tier_count < 4 {
                        let tier_colour: [u8; 4] = settings.current_preset.missing_tier_colour.unwrap();
                        return egui::Color32::from_rgb(tier_colour[0], tier_colour[1], tier_colour[2]);
                    }
                }
            }
        }

        egui::Color32::GREEN
    }

    pub fn draw_summary(&mut self, ui: &mut Ui, settings: &mut config::settings::Settings, primary_people: &Vec<PlayerData>, queued_people: &Vec<PlayerData>) {
        let combined = primary_people.iter().chain(queued_people.iter()).collect::<Vec<&PlayerData>>();
        if combined.len() == 0 {
            ui.label("A general summary of the sign-ups will be shown here.");
            return;
        }

        let mut unconfirmed = String::default();
        let mut cancelled: String = String::default();
        for player in combined.iter() {
            if player.confirmed == RAID_PLAN_UNCONFIRMED {
                unconfirmed += format!("<@{}>", player.discord_id).as_str();
            } else if player.confirmed == RAID_PLAN_CANCELLED {
                cancelled += format!("{} (<@{}>)\n", player.name, player.discord_id).as_str();
            }
        }

        if unconfirmed.len() > 0 {
            ui.label(egui::RichText::new("The following people have not confirmed their attendance on the raid plan:").color(egui::Color32::YELLOW));
            ui.label(unconfirmed);
            ui.label("");
        }

        if cancelled.len() > 0 {
            ui.label(egui::RichText::new("The following people have cancelled their attendance on the raid plan:").color(egui::Color32::RED));
            ui.label(cancelled);
            ui.label("");
        }
    }

    pub fn draw_player_info(&mut self, ui: &mut Ui, settings: &mut config::settings::Settings, expansions: &ExpansionsConfig, checked_player: &mut Option<PlayerData>) -> bool {

        let mut should_recheck = false;
        let mut player = if checked_player.is_some() {
            checked_player.clone().unwrap()
        } else {
            if self.target_player.is_some() {
                self.target_player.clone().unwrap()
            } else {
                return false;
            }
        };

        if settings.dirty_state != player.dirty_state {
            let (bad_gear, bad_socket, bad_item, embelishments) = GearChecker::check_gear(&player.character, settings, expansions);
            let pvp_gear = GearChecker::check_pvp_gear(&player.character.gear, expansions);
            let tier_count = GearChecker::check_tier_pieces(&player.character.gear, expansions);
            player.bad_gear = bad_gear;
            player.bad_socket = bad_socket;
            player.bad_special_item = bad_item;
            player.num_embelishments = embelishments;
            player.pvp_gear = pvp_gear;
            player.tier_count = tier_count;
            self.target_player = Some(player.clone());
        }

        if player.skip_reason.is_some() {
            ui.label(format!("Skipped processing {}: {}", player.name.clone(), player.skip_reason.unwrap()));
            return false;
        }

        ui.horizontal(|ui| {
            ui.add(Hyperlink::from_label_and_url("Armory", format!("{}", player.armory_url)));
            let converted_url = player.armory_url.clone().replace("worldofwarcraft.blizzard.com/en-gb", "www.warcraftlogs.com");
            ui.add(Hyperlink::from_label_and_url("Logs", format!("{}", converted_url)));
            let converted_url = player.armory_url.clone().replace("worldofwarcraft.blizzard.com/en-gb/character", "raider.io/characters");
            ui.add(Hyperlink::from_label_and_url("Raider.IO", format!("{}", converted_url)));
            if checked_player.as_ref().is_none() && ui.button("Recheck").on_hover_text("Rechecks this player.").clicked() == true {
                should_recheck = true;
            }

            if settings.current_preset.regulars.as_ref().unwrap_or(&BTreeMap::new()).get(&player.discord_id).is_none() && 
                ui.button("Add regular").on_hover_text("Marks this player as a regular, which will highlight them in the list and show a note on their player info.").clicked() {
                if settings.current_preset.regulars.is_none() {
                    settings.current_preset.regulars = Some(BTreeMap::new());
                }

                if settings.current_preset.regulars.as_ref().unwrap().get(&player.discord_id).is_some() {
                    settings.current_preset.regulars.as_mut().unwrap().remove(&player.discord_id);
                    settings.save_mut();
                } else {
                    settings.current_preset.regulars.as_mut().unwrap().insert(player.discord_id.clone(), player.name.clone());
                    settings.save_mut();
                }
            }
        });

        if let Some(regular) = settings.current_preset.regulars.as_ref().unwrap_or(&BTreeMap::new()).get(&player.discord_id) {
            ui.add(Label::new(egui::RichText::new(format!("This player is marked as a regular ({}).", regular)).color(egui::Color32::from_rgb(255, 255, 0))));
        }

        let last_updated = player.character.last_updated_timestamp.epoch / 1000;
        let last_updated: DateTime<Utc> = Utc.timestamp_opt(last_updated as i64, 0).unwrap();
        let last_updated_local: DateTime<Local> = last_updated.with_timezone(&Local);
        let now = Local::now();
        let duration = now.signed_duration_since(last_updated_local);
        let color = 
            if duration.num_days() > 2 {
                egui::Color32::from_rgb(255, 0, 0)
            } else if duration.num_days() > 0 {
                egui::Color32::from_rgb(255, 255, 0)
            } else {
                egui::Color32::from_rgb(0, 255, 0)
            };
        

        ui.label(egui::RichText::new(format!("Last armoury update for character: {}", last_updated_local.format("%A %d %b %H:%M"))).color(color));

        let max_level = if expansions.latest_expansion.is_some() {
            expansions.latest_expansion.as_ref().unwrap().max_lvl
        } else {
            90
        };

        if player.lvl < max_level {
            ui.label(egui::RichText::new(format!("{} is level {}! The current max is {}", player.name, player.lvl, max_level)).color(egui::Color32::RED));
        }

        if player.ilvl < settings.current_preset.average_ilvl {
            ui.label(format!("{} has an ilvl of {} which is below the average ilvl of {}", player.name.clone(), player.ilvl, settings.current_preset.average_ilvl));

            if player.pvp_gear {
                ui.label(egui::RichText::new("This player has PvP gear equipped, which may be the cause of the low ilvl.").color(egui::Color32::YELLOW));
            }
            
            ui.label("");
            ui.label("");
        }

        let mut boss_killed: BTreeMap<BossKey, (String, String, Vec<String>)> = BTreeMap::new();
        for raid in &player.raid_data {
            if settings.current_preset.required_raids.get(&(*raid.0 as i32)).is_some() {
                for boss in &raid.1.bosses {
                    for difficulty in &boss.1.difficulties {
                        let required_difficulties = settings.current_preset.required_raids.get(&(*raid.0 as i32)).unwrap().difficulty.get(&(*difficulty.0 as i32));
                        if required_difficulties.is_some() {
                            if required_difficulties.unwrap().boss_ids.get(boss.1.boss_id).is_some() {
                                if difficulty.1.killed_before == false {
                                    let status = boss_killed.entry(BossKey { raid_id: *raid.0, boss_id: boss.1.boss_id }).or_default();
                                    status.0 = raid.1.raid_name.clone();
                                    status.1 = boss.1.boss_name.clone();
                                    status.2.push(difficulty.1.difficulty_name.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut raid_name = String::new();
        if boss_killed.len() > 0 {
            ui.label(format!("{} has not killed the following bosses:", player.name.clone()));
            for boss in boss_killed.iter() {
                if raid_name != boss.1.0 {
                    raid_name = boss.1.0.clone();
                    ui.heading(format!("{}", raid_name));
                }

                let difficulties = boss.1.2.join(", ");
                ui.label(format!("\t{} ({})", boss.1.1, difficulties));
            }

            ui.label("");
            ui.label("");
        }

        let mut saved_bosses: BTreeMap<BossKey, (String, String, Vec<String>, u64)> = BTreeMap::new();
        for raid in &player.raid_data {
            if settings.current_preset.saved_raids.get(&(*raid.0 as i32)).is_some() {
                for boss in &raid.1.bosses {
                    for difficulty in &boss.1.difficulties {
                        let saved_difficulty = settings.current_preset.saved_raids.get(&(*raid.0 as i32)).unwrap().difficulty.get(&(*difficulty.0 as i32));
                        if saved_difficulty.is_some() {
                            if saved_difficulty.unwrap().boss_ids.get(boss.1.boss_id).is_some() {
                                if difficulty.1.boss_kill_time.is_some() {
                                    if difficulty.1.boss_kill_time.unwrap() > SavedChecker::get_wednesday_reset_timestamp() as u64 {
                                        let status = saved_bosses.entry(BossKey { raid_id: *raid.0, boss_id: boss.1.boss_id }).or_default();
                                        status.0 = raid.1.raid_name.clone();
                                        status.1 = boss.1.boss_name.clone();
                                        status.2.push(difficulty.1.difficulty_name.clone());
                                        status.3 = difficulty.1.boss_kill_time.unwrap() / 1000;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        let mut raid_name = String::new();
        if saved_bosses.len() > 0 {
            ui.label(format!("{} is saved to these bosses this reset:", player.name.clone()));
            for boss in saved_bosses.iter() {
                if raid_name != boss.1.0 {
                    raid_name = boss.1.0.clone();
                    ui.heading(format!("{}", raid_name));
                }

                let difficulties = boss.1.2.join(", ");
                let kill_time: DateTime<Utc> = Utc.timestamp_opt(boss.1.3 as i64, 0).unwrap();
                let kill_time_local: DateTime<Local> = kill_time.with_timezone(&Local);
                ui.label(format!("\t{} ({}) killed on {}", boss.1.1, difficulties, kill_time_local.format("%A %H:%M")));
            }

            ui.label("");
            ui.label("");
        }

        let gear_issue = player.bad_gear.len() > 0 || player.bad_socket.len() > 0 || player.bad_special_item.len() > 0 || (player.num_embelishments != -1 && player.num_embelishments < settings.current_preset.embelishments);
        if gear_issue {
            ui.label(format!("{} has gear that does not meet the requirements:", player.name.clone()));
        }

        if player.bad_gear.len() > 0 {
            for gear in player.bad_gear.iter() {
                ui.label(format!("\t{}", gear));
            }
        }

        if player.bad_special_item.len() > 0 {
            let special_item_colour = settings.current_preset.bad_special_item_colour.unwrap();
            for gear in player.bad_special_item.iter() {
                ui.label(egui::RichText::new(format!("\t{}", gear)).color(egui::Color32::from_rgb(special_item_colour[0], special_item_colour[1], special_item_colour[2])));
            }
        }

        if player.bad_socket.len() > 0 {
            for gear in player.bad_socket.iter() {
                ui.label(format!("\t{}", gear));
            }
        }

        if player.num_embelishments != -1 && player.num_embelishments < settings.current_preset.embelishments {
            ui.label(egui::RichText::new(format!("{} is missing {} embelishments", player.name.clone(), settings.current_preset.embelishments - player.num_embelishments)).color(egui::Color32::from_rgb(255, 0, 0)));
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
                RaidProgressStatus::Account => {
                    string = format!("{} has \"{raid_name}\" AOTC on their account, but not on this character.", player.name.clone());
                },

                RaidProgressStatus::Character => {
                    string = format!("{} has \"{raid_name}\" AOTC on this character.", player.name.clone());
                },

                RaidProgressStatus::CuttingEdge(account, character, heroic_kill) => {
                    if *account == true && *character == false {
                        if *heroic_kill == true {
                            string = format!("{} has \"{raid_name}\" Cutting Edge on their account, but on this character, they have only earned AOTC.", player.name.clone());
                        } else {
                            string = format!("{} has \"{raid_name}\" Cutting Edge on their account, but not on this character. This character has not earned AOTC.", player.name.clone());
                        }
                    } else if *account == true && *character == true {
                        if *heroic_kill == false {
                            string = format!("{} has \"{raid_name}\" Cutting Edge on this character, but has not earned AOTC on this character.", player.name.clone());
                        } else {
                            string = format!("{} has \"{raid_name}\" Cutting Edge on this character.", player.name.clone());
                        }
                        
                    }
                },

                RaidProgressStatus::EndBossKilled(killed, heroic, mythic) => {
                    if *killed == false {
                        string = format!("{} has not killed \"{raid_name}\" end boss on this character.", player.name.clone());
                    } else {
                        string = if *mythic {
                            if *heroic {
                                format!("{} has killed Mythic \"{raid_name}\" end boss on this character", player.name.clone())
                            } else {
                                format!("{} has killed Mythic \"{raid_name}\" end boss on this character, but has not done so on Heroic.", player.name.clone())
                            }
                        } else if *heroic {
                            format!("{} has killed Heroic \"{raid_name}\" end boss on this character.", player.name.clone())
                        } else {
                            format!("{} has killed Normal \"{raid_name}\" end boss on this character.", player.name.clone())
                        }
                    }
                },

                RaidProgressStatus::None => {
                    string = format!("{} does not have \"{raid_name}\" AOTC.", player.name.clone());
                },

                RaidProgressStatus::Skipped => {
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