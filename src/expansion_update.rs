use std::{fs, io::Write};
use reqwest::blocking::get;

#[derive(serde::Deserialize)]
struct MiniJsonData {
    rhcu_version: String,
    modified: u64,
}

const RHCU_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct ExpansionUpdateChecker {
    remote_data: Option<Vec<u8>>,
}

#[derive(serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
}

impl ExpansionUpdateChecker {
    pub fn new() -> Self {
        Self { remote_data: None }
    }

    pub fn need_expansion_json_update(&mut self) -> bool {
        let local_data = match fs::read("expansions.json") {
            Ok(data) => data,
            Err(_) => return self.fetch_remote_and_store(),
        };
        let local_checksum = format!("{:x}", md5::compute(&local_data));

        if !self.fetch_remote_and_store() {
            return false;
        }

        let remote_data = self.remote_data.as_ref().unwrap();
        let remote_checksum = format!("{:x}", md5::compute(remote_data));

        let local_data: MiniJsonData = serde_json::from_slice(&local_data).unwrap();
        let data: MiniJsonData = serde_json::from_slice(&remote_data).unwrap();
        local_checksum != remote_checksum && data.rhcu_version == RHCU_VERSION && data.modified > local_data.modified
    }

    fn fetch_remote_and_store(&mut self) -> bool {
        let url = "http://localhost:8000/expansions.json";
        match get(url) {
            Ok(response) => match response.bytes() {
                Ok(bytes) => {
                    self.remote_data = Some(bytes.to_vec());
                    true
                }
                Err(_) => false,
            },
            Err(_) => false,
        }
    }

    pub fn download_expansions_json(&self) -> Result<(), Box<dyn std::error::Error>> {
        let data = self.remote_data.as_ref()
            .ok_or("No data")?;

        let mut file = fs::File::create("expansions.json")?;
        file.write_all(data)?;

        Ok(())
    }

    fn need_app_update(owner: &str, repo: &str) -> bool {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            owner, repo
        );
    
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(&url)
            .header("User-Agent", "expansion-checker")
            .send()
            .ok().unwrap();
    
        if !response.status().is_success() {
            return false;
        }
    
        let data: Option<GitHubRelease> = response.text().ok().and_then(|text| serde_json::from_str(&text).ok());
        if data.is_some() {
            return data.unwrap().tag_name != RHCU_VERSION;
        }
        false
    }
}
