use std::{collections::BTreeMap, usize};

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc, Weekday};
use tracing::debug;

use crate::{checker::armory_checker::{ArmoryCharacterResponse, PlayerRaidBossData, PlayerRaidBossDifficultyData, PlayerRaidData}, config::settings::RequiredRaid};

pub struct SavedChecker {}

impl SavedChecker {
    pub fn get_wednesday_reset_timestamp() -> i64 {
        let now = Utc::now();
        let weekday = now.weekday();
    
        let days_to_subtract = match weekday {
            Weekday::Wed => 0,
            _ => (7 + weekday.num_days_from_monday() as i64 - 2) % 7,
        };
    
        let wednesday_date = now.date_naive() - Duration::days(days_to_subtract);
        let wednesday_4am = wednesday_date.and_hms_opt(4, 0, 0).unwrap();
        wednesday_4am.and_utc().timestamp_millis()
    }
    
    pub fn check_bosses(
        armory: &ArmoryCharacterResponse,
        raid_data: &mut BTreeMap<usize, PlayerRaidData>,
    ) {
        for raid in &armory.summary.raids {
            for difficulty in &raid.difficulties {
                for boss in &difficulty.bosses {
                    let raid_id = armory.summary.raids.iter().position(|x| x.name == raid.name).unwrap();
                    let difficulty_id = armory.summary.raids.get(raid_id).unwrap().difficulties.iter().position(|x| x.name == difficulty.name).unwrap();
                    let boss_id = difficulty.bosses.iter().position(|x| x.name == boss.name).unwrap();

                    let boss_data = raid_data.entry(raid_id).or_insert(PlayerRaidData { raid_name: raid.name.clone(), bosses: BTreeMap::new() })
                        .bosses.entry(boss_id).or_insert(PlayerRaidBossData { boss_id, boss_name: boss.name.clone(), difficulties: BTreeMap::new() })
                        .difficulties.entry(difficulty_id).or_insert(PlayerRaidBossDifficultyData { difficulty_id, difficulty_name: difficulty.name.clone(), boss_kill_time: None, killed_before: false });

                    boss_data.boss_kill_time = boss.last_timestamp;
                }
            }
        }
    }
}