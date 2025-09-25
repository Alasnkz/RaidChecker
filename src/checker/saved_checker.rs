use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc, Weekday};

use crate::{checker::armory_checker::ArmoryCharacterResponse, config::settings::RequiredRaid};

pub struct SavedChecker {}

impl SavedChecker {
    fn get_wednesday_reset_timestamp() -> i64 {
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
        raid_saved_check: &BTreeMap<i32, RequiredRaid>,
    ) -> Vec<(String, String)> {
        let reset_timestamp = Self::get_wednesday_reset_timestamp() as u64;
    
        raid_saved_check
            .iter()
            .filter_map(|(&raid_id, required_raid)| {
                armory.summary.raids.get(raid_id as usize).map(|armory_raid| (armory_raid, required_raid))
            })
            .flat_map(|(armory_raid, required_raid)| {
                let mut killed_bosses_in_raid: BTreeMap<usize, (String, Vec<(String, u64)>)> = BTreeMap::new();
    
                for (&difficulty_id, required_difficulty) in &required_raid.difficulty {
                    if required_difficulty.boss_ids.is_empty() {
                        continue;
                    }
    
                    if let Some(armory_difficulty) = armory_raid.difficulties.get(difficulty_id as usize) {
                        for (boss_id, armory_boss) in armory_difficulty.bosses.iter().enumerate() {
                            if required_difficulty.boss_ids.contains(&(boss_id as i32)) {
                                if let Some(timestamp) = armory_boss.last_timestamp {
                                    if timestamp > reset_timestamp {
                                        let entry = killed_bosses_in_raid
                                            .entry(boss_id)
                                            .or_insert_with(|| (armory_boss.name.clone(), Vec::new()));
                                        entry.1.push((armory_difficulty.name.clone(), timestamp / 1000));
                                    }
                                }
                            }
                        }
                    }
                }
                
                killed_bosses_in_raid.into_iter().map(move |(_, (boss_name, difficulties))| {
                    let (diff_names, timestamps): (Vec<_>, Vec<_>) = difficulties.into_iter().unzip();
                    let diff_str = diff_names.join(", ");
                    let last_kill_timestamp = timestamps.last().unwrap_or(&0);
                    let kill_time: DateTime<Utc> = Utc.timestamp_opt(*last_kill_timestamp as i64, 0).unwrap();
                    
                    (
                        armory_raid.name.clone(),
                        format!(
                            "{} ({}) @ {}",
                            boss_name,
                            diff_str,
                            kill_time.format("%A %H:%M")
                        ),
                    )
                })
            })
        .collect()
    }
}