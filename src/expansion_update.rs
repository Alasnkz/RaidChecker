use std::{fs, io::Read};

use reqwest::blocking::Request;

struct ExpansionUpdateChecker{}

impl ExpansionUpdateChecker {
    pub fn need_update() -> bool {
        // Check remote expansion config to see if it matches
        let response = reqwest::blocking::Client::new()
            .get("https://example.com/expansions.json")
            .send();
        if response.is_err() {
            return false;
        }

        let text = response.unwrap().bytes();
        if text.is_err() {
            return false;
        }

        let remote_checksum = format!("{:x}", md5::compute(text.unwrap().as_ref()));
        let local_checksum = format!("{:x}", md5::compute(fs::read("expansions.json").unwrap()));

        false
    }
}