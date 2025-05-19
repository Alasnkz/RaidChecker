use crate::config::{self};

pub(crate) struct SettingsUi {
    pub draw_item_requirements: bool,
    pub draw_raid_requirements: bool,
    pub draw_priority: bool,
    pub colour_settings: bool,
}

impl SettingsUi {
    pub fn new() -> Self {
        Self {
            draw_item_requirements: false,
            draw_raid_requirements: false,
            draw_priority: false,
            colour_settings: false,
        }
    }

    pub fn render(&mut self, ctx: &egui::Context, settings: &mut config::settings::Settings, expansions: &config::expansion_config::ExpansionsConfig) -> bool {
        let mut close: bool = false;
        egui::Window::new("Settings")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    if ui.button("Item Requirements").clicked() {
                        self.draw_item_requirements = !self.draw_item_requirements;
                    }
                    if ui.button("Raid Requirements").clicked() {
                        self.draw_raid_requirements = !self.draw_raid_requirements;
                    }
                    if ui.button("Check priority").clicked() {
                        self.draw_priority = !self.draw_priority;
                    }
                    if ui.button("Modify colours").clicked() {
                        self.colour_settings = !self.colour_settings;
                    }
                });
                if ui.button("Close").clicked() {
                    close = true;
                }
            });

        if self.draw_item_requirements {
            if Self::draw_item_requirements_settings(ctx, settings) {
                self.draw_item_requirements = false;
                settings.save_mut();
            }
        }

        if self.draw_raid_requirements {
            if Self::draw_raid_requirements_settings(ctx, settings, expansions) {
                self.draw_raid_requirements = false;
                settings.save_mut();
            }
        }

        if self.draw_priority {
            if Self::draw_check_priority(ctx, settings) {
                self.draw_priority = false;
                settings.save_mut();
            }
        }
        
        if self.colour_settings {
            if Self::draw_colour_settings(ctx, settings) {
                self.colour_settings = false;
                settings.save_mut();
            }
        }
        close
    }

    fn draw_item_requirements_settings(ctx: &eframe::egui::Context, settings: &mut config::settings::Settings) -> bool {
        let mut close: bool = false;
        egui::Window::new("Raid Item Requirements")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    for item in settings.enchantments.as_array_mut().iter_mut() {
                        ui.collapsing(item.1, |ui| {
                            ui.checkbox(&mut item.0.require_slot, "Require enchantment in slot");
                            ui.checkbox(&mut item.0.require_latest, "Require latest expansion enchantment");
                            ui.add(egui::Slider::new(&mut item.0.require_sockets, 0..=10).text("Sockets required"));
                            ui.checkbox(&mut item.0.require_greater, "Require greater enchantment").on_hover_text("Checks to see if the enchantment is a greater version of the enchantment, notable only for corruptions (TWW S2).");
                        });
                    }
                });
                if ui.button("Close").clicked() {
                    close = true;
                }
            });

        return close;
    }

    fn draw_raid_requirements_settings(ctx: &eframe::egui::Context, settings: &mut config::settings::Settings, expansion_config: &config::expansion_config::ExpansionsConfig) -> bool {
        let mut close: bool = false;
        egui::Window::new("Raid Requirements")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.add(egui::Slider::new(&mut settings.average_ilvl, 0..=1000).text("Average item level required"));

                    egui::ComboBox::from_label("Killed bosses check")
                        .selected_text(format!("{}", expansion_config.latest_expansion.as_ref().unwrap().raids.get(settings.raid_id as usize).unwrap().identifier))
                        .show_ui(ui, |ui| {
                            for raid in expansion_config.latest_expansion.as_ref().unwrap().raids.iter() {
                                ui.selectable_value(&mut settings.raid_id, raid.id, raid.identifier.clone());
                            }
                        });

                    egui::ComboBox::from_label("Selected difficulty")
                        .selected_text(format!("{}", expansion_config.latest_expansion.as_ref().unwrap().raids.get(settings.raid_id as usize).unwrap().difficulty.get(settings.raid_difficulty as usize).unwrap().difficulty_name))
                        .show_ui(ui, |ui| {
                            for difficulty in expansion_config.latest_expansion.as_ref().unwrap().raids.get(settings.raid_id as usize).unwrap().difficulty.iter() {
                                ui.selectable_value(&mut settings.raid_difficulty, difficulty.id, difficulty.difficulty_name.clone());
                            }
                        });

                    ui.horizontal(|ui| {
                        if ui.button("Enable all bosses").on_hover_ui(|ui| {
                            ui.label("Enable all bosses for this raid.");
                        }).clicked() {
                            settings.raid_difficulty_boss_id_kills.clear();
                            for i in 0..expansion_config.latest_expansion.as_ref().unwrap().raids.iter().find(|x| x.id == settings.raid_id).unwrap().boss_names.len() {
                                settings.raid_difficulty_boss_id_kills.push(i as i32);
                            }
                        };
    
                        if ui.button("Disable all bosses").on_hover_ui(|ui| {
                            ui.label("Disable all bosses for this raid.");
                        }).clicked() {
                            settings.raid_difficulty_boss_id_kills.clear();
                        };
                    });

                    let mut bid = 0;
                    for boss in expansion_config.latest_expansion.as_ref().unwrap().raids.iter().find(|x| x.id == settings.raid_id).unwrap().boss_names.iter() {
                        let mut tmp = settings.raid_difficulty_boss_id_kills.contains(&bid);
                        if ui.checkbox(&mut tmp, boss).changed() {
                            if tmp {
                                settings.raid_difficulty_boss_id_kills.push(bid);
                            } else {
                                settings.raid_difficulty_boss_id_kills.retain(|&x| x != bid);
                            }
                        }
                        bid += 1;
                    }
                });
                if ui.button("Close").clicked() {
                    close = true;
                }
            });
        return close;
    }

    fn draw_check_priority(ctx: &eframe::egui::Context, settings: &mut config::settings::Settings) -> bool {
        let mut close: bool = false;
        egui::Window::new("Modify check priority")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    let mut new_prio = settings.check_priority.clone();
                    egui::Grid::new("prio_grid").show(ui, |ui| {
                        for prio in settings.check_priority.iter() {
                            ui.label(prio.as_str());
                            // Buttons column
                            ui.horizontal(|ui| {
                                let index = new_prio.iter_mut().position(|x| x == prio).unwrap();
                                
                                if index > 0 && ui.button("up").clicked() {
                                    new_prio.swap(index, index - 1);
                                }
                                if index < new_prio.len() - 1 && ui.button("down").clicked() {
                                    new_prio.swap(index, index + 1);
                                }
                            });
                            ui.end_row();
                        }
                    });
                    settings.check_priority = new_prio;
                    ui.horizontal(|ui| {
                        if ui.button("Close").clicked() {
                            close = true;
                        }
                    });
                    
                });
            });
        return close;
    }

    fn draw_colour_settings(ctx: &eframe::egui::Context, settings: &mut config::settings::Settings) -> bool {
        let mut close: bool = false;
        egui::Window::new("Colour settings")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {

                ui.horizontal(|ui| {
                    ui.label("Skip colour");
                    let s_skip_colour = settings.skip_colour.unwrap_or([255, 0, 0, 255]);
                    let mut skip_colour = egui::Rgba::from_rgba_unmultiplied(
                        s_skip_colour[0] as f32 / 255.0,
                        s_skip_colour[1] as f32 / 255.0,
                        s_skip_colour[2] as f32 / 255.0,
                        1.0,
                    );
                
                    if egui::color_picker::color_edit_button_rgba(ui, &mut skip_colour, egui::color_picker::Alpha::Opaque).changed() {
                        settings.skip_colour = Some([
                            (skip_colour[0] * 255.0).round() as u8,
                            (skip_colour[1] * 255.0).round() as u8,
                            (skip_colour[2] * 255.0).round() as u8,
                            255,
                        ]);
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("Ilvl colour");
                    let s_ilvl_colour = settings.ilvl_colour.unwrap_or([255, 0, 0, 255]);
                    let mut ilvl_colour = egui::Rgba::from_rgba_unmultiplied(
                        s_ilvl_colour[0] as f32 / 255.0,
                        s_ilvl_colour[1] as f32 / 255.0,
                        s_ilvl_colour[2] as f32 / 255.0,
                        1.0,
                    );
                
                    if egui::color_picker::color_edit_button_rgba(ui, &mut ilvl_colour, egui::color_picker::Alpha::Opaque).changed() {
                        settings.ilvl_colour = Some([
                            (ilvl_colour[0] * 255.0).round() as u8,
                            (ilvl_colour[1] * 255.0).round() as u8,
                            (ilvl_colour[2] * 255.0).round() as u8,
                            255,
                        ]);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Saved colour");
                    let s_saved_colour = settings.saved_colour.unwrap_or([255, 0, 0, 255]);
                    let mut saved_colour = egui::Rgba::from_rgba_unmultiplied(
                        s_saved_colour[0] as f32 / 255.0,
                        s_saved_colour[1] as f32 / 255.0,
                        s_saved_colour[2] as f32 / 255.0,
                        1.0,
                    );
                
                    if egui::color_picker::color_edit_button_rgba(ui, &mut saved_colour, egui::color_picker::Alpha::Opaque).changed() {
                        settings.saved_colour = Some([
                            (saved_colour[0] * 255.0).round() as u8,
                            (saved_colour[1] * 255.0).round() as u8,
                            (saved_colour[2] * 255.0).round() as u8,
                            255,
                        ]);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Unkilled colour");
                    let s_unkilled_colour = settings.unkilled_colour.unwrap_or([255, 0, 0, 255]);
                    let mut unkilled_colour = egui::Rgba::from_rgba_unmultiplied(
                        s_unkilled_colour[0] as f32 / 255.0,
                        s_unkilled_colour[1] as f32 / 255.0,
                        s_unkilled_colour[2] as f32 / 255.0,
                        1.0,
                    );
                
                    if egui::color_picker::color_edit_button_rgba(ui, &mut unkilled_colour, egui::color_picker::Alpha::Opaque).changed() {
                        settings.unkilled_colour = Some([
                            (unkilled_colour[0] * 255.0).round() as u8,
                            (unkilled_colour[1] * 255.0).round() as u8,
                            (unkilled_colour[2] * 255.0).round() as u8,
                            255,
                        ]);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Bad gear colour");
                    let s_bad_gear_colour = settings.bad_gear_colour.unwrap_or([255, 0, 0, 255]);
                    let mut bad_gear_colour = egui::Rgba::from_rgba_unmultiplied(
                        s_bad_gear_colour[0] as f32 / 255.0,
                        s_bad_gear_colour[1] as f32 / 255.0,
                        s_bad_gear_colour[2] as f32 / 255.0,
                        1.0,
                    );
                
                    if egui::color_picker::color_edit_button_rgba(ui, &mut bad_gear_colour, egui::color_picker::Alpha::Opaque).changed() {
                        settings.bad_gear_colour = Some([
                            (bad_gear_colour[0] * 255.0).round() as u8,
                            (bad_gear_colour[1] * 255.0).round() as u8,
                            (bad_gear_colour[2] * 255.0).round() as u8,
                            255,
                        ]);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Buff colour");
                    let s_buff_colour = settings.buff_colour.unwrap_or([255, 0, 0, 255]);
                    let mut buff_colour = egui::Rgba::from_rgba_unmultiplied(
                        s_buff_colour[0] as f32 / 255.0,
                        s_buff_colour[1] as f32 / 255.0,
                        s_buff_colour[2] as f32 / 255.0,
                        1.0,
                    );
                
                    if egui::color_picker::color_edit_button_rgba(ui, &mut buff_colour, egui::color_picker::Alpha::Opaque).changed() {
                        settings.buff_colour = Some([
                            (buff_colour[0] * 255.0).round() as u8,
                            (buff_colour[1] * 255.0).round() as u8,
                            (buff_colour[2] * 255.0).round() as u8,
                            255,
                        ]);
                    }
                });
                
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        close = true;
                    }
                });
            });

        close
    }
}