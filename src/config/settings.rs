use std::{collections::BTreeMap, fs::{self, File}, io::{self, Write}, path::Path};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct EnchantmentSlotSetting {
    pub require_slot: bool,
    pub require_latest: bool,
    pub require_sockets: i32,
    #[serde(default = "default_require_greater")]
    pub require_special_item: bool,
    #[serde(default = "default_require_greater")]
    pub require_greater: bool,
    #[serde(default = "default_require_greater")]
    pub require_greater_socket: bool,
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
        }
    }
}

impl Default for EnchantmentSlotSetting {
    fn default() -> Self {
        Self {
            require_slot: false,
            require_latest: false,
            require_sockets: 0,
            require_special_item: false,
            require_greater: false,
            require_greater_socket: false,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct EnchantmentSlots {
    pub back: EnchantmentSlotSetting,
    pub chest: EnchantmentSlotSetting,
    pub foot: EnchantmentSlotSetting,
    pub hand: EnchantmentSlotSetting,
    pub head: EnchantmentSlotSetting,
    pub ring: EnchantmentSlotSetting,
    pub leg: EnchantmentSlotSetting,
    pub neck: EnchantmentSlotSetting,
    pub shoulder: EnchantmentSlotSetting,
    pub waist: EnchantmentSlotSetting,
    pub weapon: EnchantmentSlotSetting,
    pub wrist: EnchantmentSlotSetting,
}

impl EnchantmentSlots {
    // DIRTY!
    pub fn as_array_mut(&mut self) -> [(&mut EnchantmentSlotSetting, &str); 12] {
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
    pub fn as_array(&self) -> [(EnchantmentSlotSetting, &str); 12] {
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
}
impl Default for EnchantmentSlots {
    fn default() -> Self {
        Self {
            back: EnchantmentSlotSetting::default(),
            chest: EnchantmentSlotSetting::default(),
            foot: EnchantmentSlotSetting::default(),
            hand: EnchantmentSlotSetting::default(),
            head: EnchantmentSlotSetting::default(),
            ring: EnchantmentSlotSetting::default(),
            leg: EnchantmentSlotSetting::default(),
            neck: EnchantmentSlotSetting::default(),
            shoulder: EnchantmentSlotSetting::default(),
            waist: EnchantmentSlotSetting::default(),
            weapon: EnchantmentSlotSetting::default(),
            wrist: EnchantmentSlotSetting::default()
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
pub struct Settings {
    pub average_ilvl: i32,
    pub embelishments: i32,
    pub required_raids: BTreeMap<i32, RequiredRaid>,
    pub enchantments: EnchantmentSlots,
    pub skip_colour: Option<[u8; 4]>,
    pub ilvl_colour: Option<[u8; 4]>,
    pub saved_colour: Option<[u8; 4]>,
    pub unkilled_colour: Option<[u8; 4]>,
    pub bad_gear_colour: Option<[u8; 4]>,
    pub bad_socket_colour: Option<[u8; 4]>,
    pub bad_special_item_colour: Option<[u8; 4]>,
    pub buff_colour: Option<[u8; 4]>,
    #[serde(default = "default_check_priority")]
    pub check_priority: Vec<PriorityChecks>,
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
    ]
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            average_ilvl: 0,
            embelishments: 0,
            required_raids: BTreeMap::new(),
            enchantments: EnchantmentSlots::default(),
            skip_colour: Some([0xFF, 0xFF, 0x0, 0xFF]),
            ilvl_colour: Some([0x8B, 0x0, 0x0, 0xFF]),
            saved_colour: Some([0xFF, 0x0, 0x0, 0xFF]),
            unkilled_colour: Some([0xFF, 0xFF, 0x0, 0xFF]),
            bad_gear_colour: Some([0x8B, 0x0, 0x0, 0xFF]),
            bad_socket_colour: Some([0x8B, 0x0, 0x0, 0xFF]),
            bad_special_item_colour: Some([0x8B, 0x0, 0x0, 0xFF]),
            buff_colour: Some([0xFF, 0xA5, 0x0, 0xFF]),
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
                    eprintln!("Error parsing config: {}. Creating new default config.", err);
                    Self::create_default(path)
                }
            }.unwrap();

            if settings.skip_colour == None {
                settings.skip_colour = Some([0xFF, 0xFF, 0x0, 0xFF]);
            }

            if settings.ilvl_colour == None {
                settings.ilvl_colour = Some([0x8B, 0x0, 0x0, 0xFF]);
            }

            if settings.saved_colour == None {
                settings.saved_colour = Some([0xFF, 0x0, 0x0, 0xFF]);
            }

            if settings.unkilled_colour == None {
                settings.unkilled_colour = Some([0xFF, 0xFF, 0x0, 0xFF]);
            }

            if settings.bad_gear_colour == None {
                settings.bad_gear_colour = Some([0x8B, 0x0, 0x0, 0xFF]);
            }

            if settings.buff_colour == None {
                settings.buff_colour = Some([0xFF, 0xA5, 0x0, 0xFF]);
            }

            if settings.bad_socket_colour == None {
                settings.bad_socket_colour = Some([0x8B, 0x0, 0x0, 0xFF]);
            }

            if settings.bad_special_item_colour == None {
                settings.bad_special_item_colour = Some([0x8B, 0x0, 0x0, 0xFF]);
            }

            if settings.check_priority.iter().find(|x| **x == PriorityChecks::BadSocket).is_none() {
                settings.check_priority.push(PriorityChecks::BadSocket);
            }

            if settings.check_priority.iter().find(|x: &&PriorityChecks| **x == PriorityChecks::SpecialItem).is_none() {
                settings.check_priority.push(PriorityChecks::SpecialItem);
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
        let json = serde_json::to_string_pretty(self).unwrap();
        let mut file = File::create("config.json").unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }
}