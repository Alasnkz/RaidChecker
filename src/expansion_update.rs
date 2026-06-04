use std::{fs, io::Write};
use reqwest::blocking::get;
use semver::Version;
use tracing::{info, warn};

#[derive(serde::Deserialize)]
struct MiniJsonData {
    rhcu_version: String,
    modified: u64,

    changes: Option<String>,
}

const RHCU_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct ExpansionUpdateChecker {
    remote_data: Option<Vec<u8>>,
}

#[derive(serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    body: String,
}

impl ExpansionUpdateChecker {
    pub fn new() -> Self {
        Self { remote_data: None }
    }

    pub fn need_expansion_json_update(&mut self) -> (bool, String) {
        info!("Checking for expansion data update...");
        let local_data = match fs::read("expansions.json") {
            Ok(data) => data,
            Err(_) => return (self.fetch_remote_and_store(), String::new())
        };
        let local_checksum = format!("{:x}", md5::compute(&local_data));

        
        if !self.fetch_remote_and_store() {
            warn!("Failed to fetch remote expansions.json data.");
            return (false, String::new());
        }

        if self.remote_data.is_some() == false || self.remote_data.as_ref().unwrap().len() == 0 {
            warn!("Remote expansions.json data is empty or not available.");
            return (false, String::new());
        }

        let remote_data = self.remote_data.as_ref().unwrap();
        let remote_checksum = format!("{:x}", md5::compute(remote_data));

        let local_data: MiniJsonData = serde_json::from_slice(&local_data).unwrap();
        let data: MiniJsonData = serde_json::from_slice(&remote_data).unwrap();
        let remote_version = Version::parse(&data.rhcu_version).unwrap();
        let local_version = Version::parse(RHCU_VERSION).unwrap();
        info!("Local version: {local_version}, Remote version: {remote_version}, Local checksum: {local_checksum}, Remote Checksum: {remote_checksum}");
        let new = local_checksum != remote_checksum && remote_version.major == local_version.major && data.modified > local_data.modified;

        (new, local_data.changes.unwrap_or_default())
    }

    fn fetch_remote_and_store(&mut self) -> bool {
        info!("Fetching remote expansions.json data...");
        let url = "https://github.com/Alasnkz/RaidChecker/raw/refs/heads/main/expansions.json";
        match get(url) {
            Ok(response) => {
                if !response.status().is_success() || response.status() == 404 {
                    warn!("Failed to fetch expansions.json from remote: {}", response.status());
                    return false;
                }
                match response.bytes() {
                    Ok(bytes) => {
                        info!("Successfully fetched remote expansions.json data.");
                        self.remote_data = Some(bytes.to_vec());
                        true
                    }
                    Err(_) => false,
                }
            },
            _ => false,
        }
    }

    pub fn download_expansions_json(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Downloading expansions.json...");
        let data = self.remote_data.as_ref()
            .ok_or("No data")?;

        let mut file = fs::File::create("expansions.json")?;
        file.write_all(data)?;

        Ok(())
    }

    pub fn need_app_update() -> (bool, String) {
        info!("Checking for Raid Checker application update...");
        let url = format!(
            "https://api.github.com/repos/Alasnkz/RaidChecker/releases/latest",
        );

        let client = reqwest::blocking::Client::new();
        let response = client
            .get(&url)
            .header("User-Agent", "expansion-checker")
            .send()
            .ok().unwrap();
    
        if !response.status().is_success() {
            warn!("Failed to fetch the latest release information from GitHub: {}", response.status());
            return (false, String::new());
        }
    
        let data: Option<GitHubRelease> = response.text().ok().and_then(|text| serde_json::from_str(&text).ok());
        if data.is_some() {
            let data = data.unwrap();
            info!("Current version: {}, Latest version: {}", RHCU_VERSION, data.tag_name);
            return (Version::parse(&data.tag_name).unwrap() > Version::parse(RHCU_VERSION).unwrap(), data.body);
        }
        (false, String::new())
    }
}
