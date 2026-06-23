use std::{collections::{BTreeMap, HashMap, hash_map}, fs::{self, File}, io::{self, Write}, path::Path};

use tracing::error;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct SlotSetting {
    pub require_slot: bool,
    pub require_latest: bool,
    pub require_sockets: i32,
    #[serde(default = "default_true")]
    pub warn_if_socket_unfilled: bool,
    #[serde(default = "default_require_greater")]
    pub require_special_item: bool,
    #[serde(default = "default_require_greater")]
    pub require_greater: bool,
    #[serde(default = "default_require_greater")]
    pub require_greater_socket: bool,
}

fn default_true() -> bool {
    true
}

fn default_require_greater() -> bool {
    false
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub enum PriorityChecks {
    SavedKills = 0,
    Ilvl = 1,
    Enchantments = 2,
    RaidBuff = 3,
    Unkilled = 4,
    SpecialItem = 5,
    BadSocket = 6,
    MissingTier = 7,
}

impl PriorityChecks {
    pub fn as_str(&self) -> &'static str {
        match self {
            PriorityChecks::SavedKills => "Saved Kills",
            PriorityChecks::Ilvl => "Bad Item Level",
            PriorityChecks::Enchantments => "Gear enchantment issue",
            PriorityChecks::RaidBuff => "Raid Buff missing",
            PriorityChecks::Unkilled => "Bosses not killed",
            PriorityChecks::SpecialItem => "Missing Special Item",
            PriorityChecks::BadSocket => "Sockets Missing",
            PriorityChecks::MissingTier => "Missing Tier",
        }
    }
}

impl Default for SlotSetting {
    fn default() -> Self {
        Self {
            require_slot: false,
            require_latest: false,
            require_sockets: 0,
            warn_if_socket_unfilled: true,
            require_special_item: false,
            require_greater: false,
            require_greater_socket: false,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Slots {
    pub back: SlotSetting,
    pub chest: SlotSetting,
    pub foot: SlotSetting,
    pub hand: SlotSetting,
    pub head: SlotSetting,
    pub ring: SlotSetting,
    pub leg: SlotSetting,
    pub neck: SlotSetting,
    pub shoulder: SlotSetting,
    pub waist: SlotSetting,
    pub weapon: SlotSetting,
    pub wrist: SlotSetting,
}

impl Slots {
    // DIRTY!
    pub fn as_array_mut(&mut self) -> [(&mut SlotSetting, &str); 12] {
        [
            (&mut self.back, "cloak"),
            (&mut self.chest, "chest"),
            (&mut self.foot, "feet"),
            (&mut self.hand, "hand"),
            (&mut self.head, "head"),
            (&mut self.ring, "finger"),
            (&mut self.leg, "leg"),
            (&mut self.neck, "neck"),
            (&mut self.shoulder, "shoulder"),
            (&mut self.waist, "waist"),
            (&mut self.weapon, "weapon"),
            (&mut self.wrist, "wrist"),
        ]
    }

    #[allow(dead_code)]
    pub fn as_array(&self) -> [(SlotSetting, &str); 12] {
        [
            (self.back.clone(), "cloak"),
            (self.chest.clone(), "chest"),
            (self.foot.clone(), "feet"),
            (self.hand.clone(), "hand"),
            (self.head.clone(), "head"),
            (self.ring.clone(), "finger"),
            (self.leg.clone(), "leg"),
            (self.neck.clone(), "neck"),
            (self.shoulder.clone(), "shoulder"),
            (self.waist.clone(), "waist"),
            (self.weapon.clone(), "weapon"),
            (self.wrist.clone(), "wrist"),
        ]
    }

    pub fn get_by_slot_name(&self, slot_name: &str) -> Option<&SlotSetting> {
        match slot_name {
            "cloak"  => Some(&self.back),
            "chest"  => Some(&self.chest),
            "feet"   => Some(&self.foot),
            "hand"   => Some(&self.hand),
            "head"   => Some(&self.head),
            "finger" => Some(&self.ring),
            "leg"    => Some(&self.leg),
            "neck"   => Some(&self.neck),
            "shoulder" => Some(&self.shoulder),
            "waist"  => Some(&self.waist),
            "weapon" => Some(&self.weapon),
            "wrist"  => Some(&self.wrist),
            _ => None,
        }
    }
}
impl Default for Slots {
    fn default() -> Self {
        Self {
            back: SlotSetting::default(),
            chest: SlotSetting::default(),
            foot: SlotSetting::default(),
            hand: SlotSetting::default(),
            head: SlotSetting::default(),
            ring: SlotSetting::default(),
            leg: SlotSetting::default(),
            neck: SlotSetting::default(),
            shoulder: SlotSetting::default(),
            waist: SlotSetting::default(),
            weapon: SlotSetting::default(),
            wrist: SlotSetting::default()
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RequiredRaidDifficulty {
    pub boss_ids: Vec<i32>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RequiredRaid {
    pub id: i32,
    pub difficulty: BTreeMap<i32, RequiredRaidDifficulty>
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Preset {
    pub name: String,
    pub average_ilvl: i32,
    pub embelishments: i32,
    #[serde(default = "default_saved")]
    pub saved_raids: BTreeMap<i32, RequiredRaid>,
    pub required_raids: BTreeMap<i32, RequiredRaid>,
    pub slots: Slots,
    pub skip_colour: Option<[u8; 4]>,
    pub ilvl_colour: Option<[u8; 4]>,
    pub saved_colour: Option<[u8; 4]>,
    pub unkilled_colour: Option<[u8; 4]>,
    pub bad_gear_colour: Option<[u8; 4]>,
    pub bad_socket_colour: Option<[u8; 4]>,
    pub bad_special_item_colour: Option<[u8; 4]>,
    pub missing_tier_colour: Option<[u8; 4]>,
    pub buff_colour: Option<[u8; 4]>,
    #[serde(default = "default_check_priority")]
    pub check_priority: Vec<PriorityChecks>,
    pub regulars: Option<BTreeMap<String, String>>,
}

impl Default for Preset {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            average_ilvl: 0,
            embelishments: 0,
            saved_raids: BTreeMap::new(),
            required_raids: BTreeMap::new(),
            slots: Slots::default(),
            skip_colour: Some([0xFF, 0xFF, 0x0, 0xFF]),
            ilvl_colour: Some([0x8B, 0x0, 0x0, 0xFF]),
            saved_colour: Some([0xFF, 0x0, 0x0, 0xFF]),
            unkilled_colour: Some([0xFF, 0xFF, 0x0, 0xFF]),
            bad_gear_colour: Some([0x8B, 0x0, 0x0, 0xFF]),
            bad_socket_colour: Some([0x8B, 0x0, 0x0, 0xFF]),
            bad_special_item_colour: Some([0x8B, 0x0, 0x0, 0xFF]),
            missing_tier_colour: Some([218, 0, 255, 255]),
            buff_colour: Some([0xFF, 0xA5, 0x0, 0xFF]),
            regulars: None,
            check_priority: vec![
                PriorityChecks::SavedKills,
                PriorityChecks::Ilvl,
                PriorityChecks::Unkilled,
                PriorityChecks::Enchantments,
                PriorityChecks::SpecialItem,
                PriorityChecks::BadSocket,
                PriorityChecks::RaidBuff,
            ],
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct LegacySettings {
    pub average_ilvl: i32,
    pub embelishments: i32,
    #[serde(default = "default_saved")]
    pub saved_raids: BTreeMap<i32, RequiredRaid>,
    pub required_raids: BTreeMap<i32, RequiredRaid>,
    pub slots: Slots,
    pub skip_colour: Option<[u8; 4]>,
    pub ilvl_colour: Option<[u8; 4]>,
    pub saved_colour: Option<[u8; 4]>,
    pub unkilled_colour: Option<[u8; 4]>,
    pub bad_gear_colour: Option<[u8; 4]>,
    pub bad_socket_colour: Option<[u8; 4]>,
    pub bad_special_item_colour: Option<[u8; 4]>,
    pub missing_tier_colour: Option<[u8; 4]>,
    pub buff_colour: Option<[u8; 4]>,
    #[serde(default = "default_check_priority")]
    pub check_priority: Vec<PriorityChecks>,
    pub regulars: Option<BTreeMap<String, String>>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Settings {
    pub presets: HashMap<String, Preset>,
    pub last_preset: Option<String>,

    #[serde(skip)]
    pub current_preset: Preset,

    #[serde(skip)]
    pub dirty_state: i32
}

fn default_saved() -> BTreeMap<i32, RequiredRaid> {
    BTreeMap::new()
}

fn default_check_priority() -> Vec<PriorityChecks> {
    vec![
        PriorityChecks::SavedKills,
        PriorityChecks::Ilvl,
        PriorityChecks::Unkilled,
        PriorityChecks::Enchantments,
        PriorityChecks::SpecialItem,
        PriorityChecks::BadSocket,
        PriorityChecks::RaidBuff,
        PriorityChecks::MissingTier,
    ]
}

impl Default for Settings {
    fn default() -> Self {
        let mut setting = Self {
            presets: HashMap::new(),
            last_preset: None,
            current_preset: Preset::default(),
            dirty_state: 0
        };

        setting.presets.insert("Default".to_owned(), Preset::default());
        setting.last_preset = Some("Default".to_owned());
        setting.current_preset = setting.presets.get("Default").unwrap().clone();

        setting
    }
}

impl Settings {
    fn create_default<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let settings = Settings::default();
        let json = serde_json::to_string_pretty(&settings).unwrap();
        let mut file = File::create(path).unwrap();
        file.write_all(json.as_bytes()).unwrap();
        println!("Default config created.");
        Ok(settings)
    }

    pub fn read_or_create<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        // Try to read the file
        if path.as_ref().exists() {
            let content = fs::read_to_string(&path).unwrap();
            let mut settings = match serde_json::from_str(&content) {
                Ok(config) => { Ok(config) }

                Err(err) => {
                    let legacy = serde_json::from_str(&content);
                    if legacy.is_err() {
                        println!("Error reading config file: {:?}. Creating default config.", legacy.err());
                        Self::create_default(path)
                    } else {
                        let legacy_settings: LegacySettings = legacy.unwrap();
                        let mut new_settings = Settings::default();
                        let preset = Preset {
                            name: "Default".to_owned(),
                            average_ilvl: legacy_settings.average_ilvl,
                            embelishments: legacy_settings.embelishments,
                            saved_raids: legacy_settings.saved_raids,
                            required_raids: legacy_settings.required_raids,
                            slots: legacy_settings.slots,
                            skip_colour: legacy_settings.skip_colour,
                            ilvl_colour: legacy_settings.ilvl_colour,
                            saved_colour: legacy_settings.saved_colour,
                            unkilled_colour: legacy_settings.unkilled_colour,
                            bad_gear_colour: legacy_settings.bad_gear_colour,
                            bad_socket_colour: legacy_settings.bad_socket_colour,
                            bad_special_item_colour: legacy_settings.bad_special_item_colour,
                            missing_tier_colour: legacy_settings.missing_tier_colour,
                            buff_colour: legacy_settings.buff_colour,
                            check_priority: legacy_settings.check_priority,
                            regulars: legacy_settings.regulars
                        };
                        new_settings.presets.insert("Default".to_owned(), preset);
                        new_settings.last_preset = Some("Default".to_owned());
                        Ok(new_settings)
                    }

                }
            }.unwrap();

            settings.current_preset = settings.presets.get(settings.last_preset.as_ref().unwrap()).unwrap().clone();

            if settings.current_preset.skip_colour == None {
                settings.current_preset.skip_colour = Some([0xFF, 0xFF, 0x0, 0xFF]);
            }

            if settings.current_preset.ilvl_colour == None {
                settings.current_preset.ilvl_colour = Some([0x8B, 0x0, 0x0, 0xFF]);
            }

            if settings.current_preset.saved_colour == None {
                settings.current_preset.saved_colour = Some([0xFF, 0x0, 0x0, 0xFF]);
            }

            if settings.current_preset.unkilled_colour == None {
                settings.current_preset.unkilled_colour = Some([0xFF, 0xFF, 0x0, 0xFF]);
            }

            if settings.current_preset.bad_gear_colour == None {
                settings.current_preset.bad_gear_colour = Some([0x8B, 0x0, 0x0, 0xFF]);
            }

            if settings.current_preset.buff_colour == None {
                settings.current_preset.buff_colour = Some([0xFF, 0xA5, 0x0, 0xFF]);
            }

            if settings.current_preset.bad_socket_colour == None {
                settings.current_preset.bad_socket_colour = Some([0x8B, 0x0, 0x0, 0xFF]);
            }

            if settings.current_preset.bad_special_item_colour == None {
                settings.current_preset.bad_special_item_colour = Some([0x8B, 0x0, 0x0, 0xFF]);
            }

            if settings.current_preset.missing_tier_colour == None {
                settings.current_preset.missing_tier_colour = Some([218, 0, 255, 255]);
            }

            if settings.current_preset.check_priority.iter().find(|x| **x == PriorityChecks::BadSocket).is_none() {
                settings.current_preset.check_priority.push(PriorityChecks::BadSocket);
            }

            if settings.current_preset.check_priority.iter().find(|x: &&PriorityChecks| **x == PriorityChecks::SpecialItem).is_none() {
                settings.current_preset.check_priority.push(PriorityChecks::SpecialItem);
            }

            if settings.current_preset.check_priority.iter().find(|x: &&PriorityChecks| **x == PriorityChecks::MissingTier).is_none() {
                settings.current_preset.check_priority.push(PriorityChecks::MissingTier);
            }
            Ok(settings)
        } else {
            Self::create_default(path)
        }
    }

    pub fn save(&self) {
        let json = serde_json::to_string_pretty(self).unwrap();
        let mut file = File::create("config.json").unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }

    pub fn save_mut(&mut self) {
        *self.presets.get_mut(self.last_preset.as_ref().unwrap()).unwrap() = self.current_preset.clone();
        let json = serde_json::to_string_pretty(self).unwrap();
        let mut file = File::create("config.json").unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }
}