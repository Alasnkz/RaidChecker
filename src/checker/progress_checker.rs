use std::collections::BTreeMap;

use regex::Regex;
use reqwest::blocking::Client;
use tracing::{error, info};
use crate::{checker::armory_checker::{ArmoryCharacterAchievementResponse, ArmoryCharacterResponse, ArmoryRaids, RaidProgressStatus}, config::{self, expansion_config::RaidAchievements, settings::{RequiredRaid, RequiredRaidDifficulty}}};

pub struct ProgressChecker {}

impl ProgressChecker {
    pub fn check_aotc(
        url: String,
        armory: &ArmoryCharacterResponse,
        expansions: &config::expansion_config::ExpansionsConfig,
        raid_saved_check_input: &BTreeMap<i32, RequiredRaid>,
    ) -> BTreeMap<i32, (String, RaidProgressStatus)> {
        info!("--- AOTC CHECK ---");

        let raid_saved_check = Self::determine_raids_to_check(expansions, raid_saved_check_input);

        let feats_url = format!("{}/achievements/feats-of-strength", url.trim_end_matches('/'));
        let response_text = Self::fetch_achievements(&feats_url);

        let mut aotc_ce_status = BTreeMap::new();
        if let Some(data) = Self::extract_achievement_data(&response_text) {
            Self::process_achievements(
                &data,
                armory,
                expansions,
                &raid_saved_check,
                &mut aotc_ce_status,
            );
        } else {
            error!("Could not find character profile initial state in response.");
        }

        Self::fill_missing_raids(&raid_saved_check, expansions, &mut aotc_ce_status);

        aotc_ce_status
    }

    fn determine_raids_to_check(
        expansions: &config::expansion_config::ExpansionsConfig,
        input: &BTreeMap<i32, RequiredRaid>,
    ) -> BTreeMap<i32, RequiredRaid> {
        if input.is_empty()
            || input.iter().all(|x| x.1.difficulty.is_empty())
            || input
                .iter()
                .all(|x| x.1.difficulty.iter().all(|y| y.1.boss_ids.is_empty()))
        {
            info!("Specified raid is empty, assuming last raid.");
            if let Some(latest) = &expansions.latest_expansion {
                if let Some(season) = &latest.latest_season {
                    if let Some(last_raid) = season.raids.last() {
                        return BTreeMap::from([(
                            last_raid.id,
                            RequiredRaid {
                                id: last_raid.id,
                                difficulty: BTreeMap::from([(
                                    1,
                                    RequiredRaidDifficulty { boss_ids: vec![0] },
                                )]),
                            },
                        )]);
                    }
                }
            }
        }
        input.clone()
    }

    fn fetch_achievements(url: &str) -> String {
        let client = Client::new();
        client
            .get(url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36",
            )
            .send()
            .and_then(|r| r.text())
            .unwrap_or_else(|e| {
                error!("Failed to fetch achievements: {}", e);
                String::new()
            })
    }

    fn extract_achievement_data(response: &str) -> Option<ArmoryCharacterAchievementResponse> {
        let re = Regex::new(r#"var\s+characterProfileInitialState\s*=\s*(\{.*?\});"#).unwrap();
        re.captures(response)
            .and_then(|cap| serde_json::from_str(&cap[1]).ok())
    }

    fn process_achievements(
        data: &ArmoryCharacterAchievementResponse,
        armory: &ArmoryCharacterResponse,
        expansions: &config::expansion_config::ExpansionsConfig,
        raids_to_check: &BTreeMap<i32, RequiredRaid>,
        aotc_ce_status: &mut BTreeMap<i32, (String, RaidProgressStatus)>,
    ) {
        // Map achievements by ID for quick lookup
        let mut achievement_map = BTreeMap::new();
        for category in &data.achievement_category.subcategories {
            if category.1.id == "raids" {
                for achievement in &category.1.achievements {
                    achievement_map.insert(achievement.id, achievement);
                }
                break;
            }
        }
    
        for (&raid_id, _) in raids_to_check {
            let raid = expansions
                .latest_expansion
                .as_ref()
                .and_then(|e| e.find_raid_by_id(raid_id))
                .expect("Raid not found in expansion data");
            let raid_name = raid.identifier.clone();
    
            let status = if raid.achievements.dependency_id != -1 {
                let raid_summary = armory.summary.raids.get(raid_id as usize);
                Self::end_boss_status(raid_summary)
            } else {
                let achievement = achievement_map.get(&raid.achievements.aotc)
                    .or_else(|| achievement_map.get(&raid.achievements.ce));
                if let Some(ach) = achievement {
                    Self::aotc_ce_status(ach.id, &raid.achievements, armory.summary.raids.get(raid_id as usize))
                } else {
                    RaidProgressStatus::None
                }
            };
    
            aotc_ce_status.entry(raid_id).or_insert((raid_name, status));
        }
    }
    
    fn aotc_ce_status(
        earned_achievement_id: i32,
        achievements: &RaidAchievements,
        raid: Option<&ArmoryRaids>,
    ) -> RaidProgressStatus {
        if raid.is_none() {
            info!("No raid summary! Depending on purely achievement.");
            return if earned_achievement_id == achievements.ce {
                RaidProgressStatus::CuttingEdge(true, false, false)
            } else {
                RaidProgressStatus::Account
            };
        }

        let raid = raid.unwrap();
        let mut has_cutting_edge = false;
    
        if earned_achievement_id == achievements.ce {
            if let Some(mythic) = raid.difficulties.get(3) {
                if let Some(last_boss) = mythic.bosses.last() {
                    if last_boss.kill_count >= 1 {
                        info!("Character has killed mythic end boss.");
                        has_cutting_edge = true;
                    }
                }
            }
        }
    
        if let Some(heroic) = raid.difficulties.get(2) {
            if let Some(last_boss) = heroic.bosses.last() {
                if last_boss.kill_count >= 1 {
                    return if earned_achievement_id == achievements.ce {
                        RaidProgressStatus::CuttingEdge(true, has_cutting_edge, true)
                    } else if earned_achievement_id == achievements.aotc {
                        RaidProgressStatus::Character
                    } else {
                        RaidProgressStatus::None
                    };
                }
            }
        }
    
        if earned_achievement_id == achievements.ce && has_cutting_edge {
            return RaidProgressStatus::CuttingEdge(true, has_cutting_edge, false);
        }
    
        RaidProgressStatus::Account
    }


    fn end_boss_status(
        raid: Option<&ArmoryRaids>
    ) -> RaidProgressStatus {
        if raid.is_none() {
            return RaidProgressStatus::Error;
        }

        let raid = raid.unwrap();
        info!("Checking end boss kill for {}", raid.name);
        let mythic_killed = raid.difficulties.get(3)
            .and_then(|d| d.bosses.last())
            .map(|b| b.kill_count >= 1)
            .unwrap_or(false);
    
        let heroic_killed = raid.difficulties.get(2)
             .and_then(|d| d.bosses.last())
            .map(|b| b.kill_count >= 1)
            .unwrap_or(false);
    
        RaidProgressStatus::EndBossKilled(heroic_killed || mythic_killed, heroic_killed, mythic_killed)
    }

    fn fill_missing_raids(
        raids_to_check: &BTreeMap<i32, RequiredRaid>,
        expansions: &config::expansion_config::ExpansionsConfig,
        aotc_ce_status: &mut BTreeMap<i32, (String, RaidProgressStatus)>,
    ) {
        for (&raid_id, required) in raids_to_check {
            if !aotc_ce_status.contains_key(&raid_id)
                && required
                    .difficulty
                    .iter()
                    .any(|(_, diff)| !diff.boss_ids.is_empty())
            {
                if let Some(raid) = expansions
                    .latest_expansion
                    .as_ref()
                    .and_then(|e| e.find_raid_by_id(raid_id))
                {
                    info!("No AOTC/CE data found for raid {}", raid.identifier);
                    aotc_ce_status.insert(raid_id, (raid.identifier.clone(), RaidProgressStatus::None));
                }
            }
        }
    }
}