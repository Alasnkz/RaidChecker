use reqwest::blocking::Client;
use std::{sync::mpsc::{self, Receiver, Sender}, thread};

use crate::config::{self, last_raid::{LastRaid}};

use super::check_player::{PlayerChecker, PlayerData};

// When called
// Register async messagers
// Everything happens in seperate thread
// UI calls are done to send data such as progress
// WHen search is required we'll lock and await main thread to handle search UI



#[derive(serde::Deserialize, Clone)]
pub struct Player {
    pub specName: Option<String>,
    pub name: String,
    pub roleName: Option<String>,
    pub className: String,
    pub userId: String,
    pub status: String
}

#[derive(serde::Deserialize)]
struct RaidHelper {
    #[serde(alias = "displayTitle")]
    name: String,
    signUps: Vec<Player>
}

#[derive(PartialEq)]
pub enum RaidSheetState {
    None,
    Init, // Getting raid helper data
    Error(String), // Error, killing the attempt.
    Checking(String), // Checking players
    Search((String, Option<String>, Vec<(String, String)>)), // Armory search
    Question(String), // Asking a question
    QuestionStringSkip(String), // Asking a question involving a string (but also including a Skip button)
    Wait, // Getting data
}

pub struct RaidSheet {
   pub(crate) state: RaidSheetState,
   pub(crate) ui_sender: Sender<RaidHelperUIStatus>,
   pub(crate) ui_reciever: Receiver<RaidHelperCheckerStatus>,
   pub(crate) search_filter: String,
   pub(crate) question_string: String,
   pub(crate) wait_counter: usize,
   pub(crate) frame_counter: usize,

   // Player stuff
   pub(crate) active_players: Vec<PlayerData>,
   pub(crate) queued_players: Vec<PlayerData>,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            specName: Some("N/A".to_string()),
            name: String::default(),
            roleName: None,
            className: String::default(),
            userId: String::default(),
            status: String::default()
        }
    }
}
impl Default for RaidSheet {
    fn default() -> Self {
        let (tx, _rx) = mpsc::channel();
        let (_tx2, rx2) = mpsc::channel();
        Self {
            state: RaidSheetState::None,
            ui_sender: tx,
            ui_reciever: rx2,
            search_filter: String::default(),
            question_string: String::default(),
            wait_counter: 0,
            frame_counter: 0,

            active_players: Vec::new(),
            queued_players: Vec::new(),
        }
    }
}


pub enum RaidHelperUIStatus {
    Answer(bool),
    AnswerStringSkip(Option<String>),
    SearchResponse(Option<(String, String)>),
    SearchResponseNewName(),
    SearchResponseSkip(),
}

#[derive(Debug)]
pub enum RaidHelperCheckerStatus {
    Error(String),
    Checking(String),
    Search((String, Option<String>, Vec<(String, String)>)),
    Question(String),
    QuestionStringSkip(String),
    CheckResults(LastRaid),
    PlayerResult(PlayerData),
}

fn should_check_player(player: &Player) -> bool {
    let valid_status = player.status == "primary" || player.status == "queued";
    let valid_role = match player.roleName.clone().unwrap_or("".to_string()).as_str().to_lowercase().as_str() {
        "tanks" | "healers" | "ranged" | "melee" => true,
        _ => false,
    };

    let valid_class = match player.className.to_lowercase().as_str() {
        "tank" | "healer" | "ranged" | "melee" | "dps" => true,
        _ => false,
    };

    valid_status && (valid_class || valid_role) && player.className.to_lowercase() != "tentative"
}

impl RaidSheet {
    pub fn init_from_last_raid(&mut self, last_raid: &LastRaid) {
        self.state = RaidSheetState::None;
        self.active_players.clear();
        self.queued_players.clear();
        for player in last_raid.players.iter() {
            if player.queued == false {
                self.active_players.push(player.clone());
            } else {
                self.queued_players.push(player.clone());
            }
        }
    }

    pub fn init(&mut self, url: String, is_player_only: bool, settings: config::settings::Settings, expansions: config::expansion_config::ExpansionsConfig, realms: config::realms::RealmJson,
        raid_id: i32, raid_difficulty: i32, boss_kills: Vec<i32>, check_saved_prev_difficulty: bool, mut last_raid: LastRaid)
    {
        let (uis, thread_reciever) = mpsc::channel();
        let (thread_sender, uir) = mpsc::channel();
        self.ui_sender = uis;
        self.ui_reciever = uir;
        self.state = RaidSheetState::Init;

        if is_player_only == true {
            thread::spawn(move || {
                let _ = thread_sender.send(RaidHelperCheckerStatus::Checking(format!("player {}", url.clone())));
                let mut player = Player::default();
                player.name = url.clone();

                let player_data = PlayerChecker::check_player(&player, &thread_sender, &thread_reciever, &settings, &expansions, &realms, raid_id, raid_difficulty, &boss_kills, check_saved_prev_difficulty, None);
                if player_data.is_some() {
                    let _ = thread_sender.send(RaidHelperCheckerStatus::PlayerResult(player_data.unwrap()));
                } else {
                    let _ = thread_sender.send(RaidHelperCheckerStatus::Error(format!("Could not find player {:?}", url.clone())));
                }
            });
            return;
        }


        thread::spawn(move || {
            let client = Client::new();
            let response = client
                .get(url.clone())
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3")
                .send();

            if response.is_err() {
                let _ = thread_sender.send(RaidHelperCheckerStatus::Error(format!("Failed to get response: {:?}", response.err())));
                return;
            }

            let text = response.unwrap().text();
            if text.is_err() {
                let _ = thread_sender.send(RaidHelperCheckerStatus::Error(format!("Failed to get response text: {:?}", text.err())));
                return;
            }

            let raid_response_res: Result<RaidHelper, serde_json::Error> = serde_json::from_str(&text.unwrap());
            if raid_response_res.is_err() {
                let _ = thread_sender.send(RaidHelperCheckerStatus::Error(format!("Failed to parse response: {:?}", raid_response_res.err())));
                return;
            }

            let raid_response = raid_response_res.unwrap();
            let mut count = 1;

            if last_raid.raid_url != url {
                last_raid.raid_url = String::new();
                last_raid.players.clear();
            }

            let viable: Vec<&Player> = raid_response.signUps.iter().filter(|x| should_check_player(x)).collect();

            let mut vec_player = Vec::new();
            for player in viable.iter() {
                let player_url = if last_raid.players.iter().find(|x| x.discord_id == player.userId && x.name == player.name).is_some() {
                    Some(last_raid.players.iter().find(|x| x.discord_id == player.userId).unwrap().armory_url.clone())
                } else {
                    None
                };

                let _ = thread_sender.send(RaidHelperCheckerStatus::Checking(format!("{} {}/{}", player.name, count, viable.len())));
                let player_data = PlayerChecker::check_player(player, &thread_sender, &thread_reciever, &settings, &expansions, &realms, raid_id, raid_difficulty, &boss_kills, check_saved_prev_difficulty, player_url);
                if player_data.is_some() {
                    vec_player.push(player_data.unwrap());
                } else {
                    vec_player.push(PlayerData {
                        discord_id: player.userId.clone(),
                        name: player.name.clone(),
                        status: player.status.clone(),
                        unkilled_bosses: Vec::new(),
                        bad_gear: Vec::new(),
                        ilvl: 0,
                        saved_bosses: Vec::new(),
                        aotc_status: super::armory_checker::AOTCStatus::None,
                        buff_status: 0,
                        buff_possible: false,
                        skip_reason: Some("Could not find player".to_owned()),
                        armory_url: "".to_owned(),
                        queued: player.status.to_lowercase() != "primary" || player.className.to_lowercase() == "bench" 
                    });
                }
                
                count += 1;
            }

            last_raid.players = vec_player.clone();
            last_raid.raid_url = url.clone();
            last_raid.raid_name = raid_response.name.clone();
            last_raid.save();

            let _ = thread_sender.send(RaidHelperCheckerStatus::CheckResults(LastRaid {
                raid_url: url,
                raid_name: raid_response.name,
                players: vec_player
            }));
       });
    }

    fn update_wait_state(&mut self, ctx: &egui::Context) {
        self.frame_counter += 1;
    
        if self.frame_counter % 10 == 0 {
            self.wait_counter = (self.wait_counter + 1) % 4;
        }
    
        ctx.request_repaint();
    }

    pub fn draw(&mut self, ctx: &egui::Context, last_raid: &mut config::last_raid::LastRaid, just_checked: &mut bool,
        checked_player: &mut Option<PlayerData>) {

        let message = match self.ui_reciever.try_recv() {
            Ok(msg) => Some(msg),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => None
        };

        if message.is_some() {
            match message.unwrap() {
                
                RaidHelperCheckerStatus::Error(msg) => {
                    self.state = RaidSheetState::Error(msg);
                },

                RaidHelperCheckerStatus::Checking(msg) => {
                    self.state = RaidSheetState::Checking(msg);
                },

                RaidHelperCheckerStatus::Search(msg) => {
                    self.state = RaidSheetState::Search(msg);
                    self.search_filter = String::default();
                },

                RaidHelperCheckerStatus::Question(msg) => {
                    self.state = RaidSheetState::Question(msg);
                },

                RaidHelperCheckerStatus::QuestionStringSkip(msg) => {
                    self.state = RaidSheetState::QuestionStringSkip(msg);
                    self.question_string = String::default();
                },

                RaidHelperCheckerStatus::CheckResults(results) => {
                    self.active_players.clear();
                    self.queued_players.clear();
                    *last_raid = results.clone();
                    
                    for player in last_raid.players.iter() {
                        if player.queued == false {
                            self.active_players.push(player.clone() as PlayerData);
                        } else {
                            self.queued_players.push(player.clone());
                        }
                    }
                    *just_checked = true;
                    self.state = RaidSheetState::None;
                }

                RaidHelperCheckerStatus::PlayerResult(player) => {
                    *checked_player = Some(player.clone());
                    self.state = RaidSheetState::None;
                }
            }
        }

        let mut wait = false;
        match &self.state {
            RaidSheetState::Init => {
                egui::Window::new("Raid Helper - Getting Raid Helper Data")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("Initializing...");
                    });
            },

            RaidSheetState::Checking(_msg) => {
                self.update_wait_state(ctx);
                egui::Window::new("Raid Helper - Parsing Players")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        let dots = ".".repeat(self.wait_counter);
                        ui.label(match &self.state {
                            RaidSheetState::Checking(msg) => format!("Checking {}{}", msg, dots),
                            _ => "No data yet.".to_owned()
                        });
                    });
            },
            
            RaidSheetState::Search(msg) => {
                let (name, spec, results) = msg;
                egui::Window::new("Raid Helper - Searching Armory")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(format!("Search results for \"{}\":", match spec {
                            Some(spec) => format!("{}\" signed as spec \"{}", name, spec),    
                            None => format!("{}", name)
                        }));

                        ui.horizontal(|ui| {
                            ui.label("Filter: ");
                            ui.text_edit_singleline(&mut self.search_filter);
                        });
                        
                        let filtered_results: Vec<(String, String)> = results
                            .iter()
                            .filter(|(name, _)| name.to_lowercase().contains(&self.search_filter.to_lowercase()))
                            .cloned()
                            .collect();
                        
                        ui.horizontal(|ui| {
                            if ui.button("Skip check").on_hover_text("Skip this player").clicked() {
                                let _ = self.ui_sender.send(RaidHelperUIStatus::SearchResponseSkip());
                                wait = true;
                            }

                            if ui.button("New name").on_hover_text("Search for a new name").clicked() {
                                let _ = self.ui_sender.send(RaidHelperUIStatus::SearchResponseNewName());
                                wait = true;
                            }
                        });

                        for (name, realm) in filtered_results.iter() {
                            if ui.button(name).clicked() {
                                let _ = self.ui_sender.send(RaidHelperUIStatus::SearchResponse(Some((name.clone(), realm.clone()))));
                                wait = true;
                            }
                        }
                    });
            },

            RaidSheetState::Error(_msg) => {
                egui::Window::new("Raid Helper - Error")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        if ui.label(match &self.state {
                            RaidSheetState::Error(msg) => format!("Error: {}", msg),
                            _ => "No data yet.".to_owned()
                        }).clicked() {
                            self.state = RaidSheetState::None;
                        }
                    });
            },

            RaidSheetState::Question(_msg) => {
                egui::Window::new("Raid Helper - Question")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(match &self.state {
                            RaidSheetState::Question(msg) => format!("Question: {}", msg),
                            _ => "No data yet.".to_owned()
                        });
                        if ui.button("Yes").clicked() {
                            let _ = self.ui_sender.send(RaidHelperUIStatus::Answer(true));
                        }
                        if ui.button("No").clicked() {
                            let _ = self.ui_sender.send(RaidHelperUIStatus::Answer(false));
                        }
                    });
            },
            
            RaidSheetState::QuestionStringSkip(MAX_COMPUTE_SHADER_STORAGE_BLOCKSmsg) => {
                egui::Window::new("Raid Helper - Question")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(match &self.state {
                            RaidSheetState::QuestionStringSkip(msg) => format!("{}", msg),
                            _ => "No data yet.".to_owned()
                        });

                        let response = ui.text_edit_singleline(&mut self.question_string);
                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            let _ = self.ui_sender.send(RaidHelperUIStatus::AnswerStringSkip(Some(self.question_string.clone())));
                            self.state = RaidSheetState::Wait;
                        }

                        ui.horizontal(|ui| {
                            if ui.button("Submit").clicked() {
                                let _ = self.ui_sender.send(RaidHelperUIStatus::AnswerStringSkip(Some(self.question_string.clone())));
                                self.state = RaidSheetState::Wait;
                            }
    
                            if ui.button("Skip").clicked() {
                                let _ = self.ui_sender.send(RaidHelperUIStatus::AnswerStringSkip(None));
                                self.state = RaidSheetState::Wait;
                            }
                        }); 
                    });
            },

            RaidSheetState::Wait => {
                self.update_wait_state(ctx);
                egui::Window::new("Raid Helper - Waiting")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        let dots = ".".repeat(self.wait_counter);
                        ui.label(format!("Please wait{}", dots));
                    });
            },

            _ => {
                egui::Window::new("Raid Helper")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("No data yet.");
                    });
            }
        }

        if wait == true {
            self.state = RaidSheetState::Wait;
        }
    }
}