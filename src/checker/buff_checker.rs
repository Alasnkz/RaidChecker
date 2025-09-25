use std::collections::BTreeMap;
use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;
use reqwest::blocking::Client;
use tracing::info;
use crate::{checker::armory_checker::ArmoryCharacterReputationResponse, config::{expansion_config::ExpansionsConfig, settings::{RequiredRaid, RequiredRaidDifficulty}}};

pub struct BuffChecker {}

impl BuffChecker {
    pub fn check_raids(
        _url: String,
        expansions: &ExpansionsConfig,
        raid_saved_check_input: &BTreeMap<i32, RequiredRaid>,
    ) -> Result<BTreeMap<i32, (String, i32, bool, i32, i32)>> {
        info!("Checking for raid buffs");
    
        let latest_expansion = expansions
            .latest_expansion
            .as_ref()
            .context("Latest expansion configuration is missing")?;

        let raid_saved_check = if raid_saved_check_input.values().all(|r| r.difficulty.values().all(|d| d.boss_ids.is_empty())) {
            info!("No valid raid difficulties specified, assuming last raid of the latest season.");
            let latest_raid = latest_expansion
                .latest_season
                .as_ref()
                .and_then(|s| s.raids.last())
                .context("Could not find the latest raid in configuration")?;
            
            BTreeMap::from([(
                latest_raid.id,
                RequiredRaid {
                    id: latest_raid.id,
                    difficulty: BTreeMap::from([(1, RequiredRaidDifficulty { boss_ids: vec![0] })]),
                },
            )])
        } else {
            raid_saved_check_input.clone()
        };
    
        let url = format!("{}/reputation", _url.trim_end_matches('/'));
        let client = Client::new();
        let response_text = client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36")
            .send()
            .context("Failed to send request to armory")?
            .text()
            .context("Failed to read armory response text")?;
    
        let re = Regex::new(r#"var\s+characterProfileInitialState\s*=\s*(\{.*?\});"#)?;
        let js_variable = re
            .captures(&response_text)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str())
            .context("Could not find 'characterProfileInitialState' JSON in the response")?;
        
        let armory_response: ArmoryCharacterReputationResponse = serde_json::from_str(js_variable)
            .context("Failed to deserialize character JSON data")?;
    
        let mut raid_buffs = BTreeMap::new();
        let all_reputations: Vec<_> = armory_response.reputations.reputations
            .iter()
            .flat_map(|cat| cat.reputations.iter())
            .flat_map(|rep| std::iter::once(rep).chain(rep.reputations.iter()))
            .collect();
    
        for (raid_id, raid_config) in &raid_saved_check {
            if raid_config.difficulty.values().all(|d| d.boss_ids.is_empty()) {
                info!("Skipping raid ID: {} as it has no specified boss IDs to check.", raid_id);
                continue;
            }
    
            let raid = latest_expansion.find_raid_by_id(*raid_id).with_context(|| format!("Configuration for raid ID {} not found", raid_id))?;
            let Some(reputation) = &raid.reputation else {
                info!("Raid {} has no reputation assigned, skipping buff check.", raid.identifier);
                raid_buffs.insert(*raid_id, (raid.identifier.clone(), 0, false, 0, 0));
                continue;
            };

            let character_rep_data = all_reputations.iter().find(|&&rep| rep.id == reputation.raid_rep_slug);
            let (current_renown, current_renown_amount) = if let Some(data) = character_rep_data {
                let renown_str = data.standing.as_deref().unwrap_or("Renown 0").split(' ').last().unwrap_or("0");
                let level = renown_str.parse().unwrap_or(0);
                let amount = data.value.unwrap_or(0) as i32;
                info!("Reputation data found for {}: Renown {}, Amount {}", raid.identifier, level, amount);
                (level, amount)
            } else {
                info!("No reputation data found for raid: {}. Assuming Renown 1.", raid.identifier);
                (1, 0)
            };

            let start_time = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp_opt(reputation.renown_start, 0).unwrap(), Utc);
            let max_renown = (Utc::now() - start_time).num_weeks() + 2;

            let missing_buff_levels: Vec<_> = reputation.raid_buff_renowns
                .iter()
                .filter(|&&lvl| lvl > current_renown && (lvl as i64) <= max_renown)
                .copied()
                .collect();
            
            if missing_buff_levels.is_empty() {
                info!("No missing buff levels found for {}, current renown: {}, max renown: {}", raid.identifier, current_renown, max_renown);
                continue;
            }

            let next_buff_renown = *missing_buff_levels.first().unwrap();
            let renown_levels_to_gain = next_buff_renown - current_renown;

            let possible = if character_rep_data.is_some() {
                let weekly_cap = reputation.max_renown_value_weekly + current_renown_amount; // Cap includes current progress.
                let points_needed = renown_levels_to_gain * reputation.renown_level_value;
                (points_needed as f32) <= (weekly_cap as f32)
            } else {
                let points_needed = renown_levels_to_gain * reputation.renown_level_value;
                (points_needed as f32) <= (reputation.max_renown_value_weekly as f32)
            };
            
            info!("{} Missing buff renowns: {:?}, possible to get a buff: {possible}", raid.identifier, missing_buff_levels);
            raid_buffs.insert(
                *raid_id,
                (
                    raid.identifier.clone(),
                    missing_buff_levels.len() as i32,
                    possible,
                    reputation.buff_size,
                    reputation.max_renown_value_weekly,
                ),
            );
        }
    
        Ok(raid_buffs)
    }
}