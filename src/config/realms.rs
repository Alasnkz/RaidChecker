use std::fs::{File};
use std::path::Path;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct RealmLinks {}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct RealmKey {}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Realms {
    #[serde(skip_deserializing)]
    key: Option<RealmKey>,
    pub name: String,
    id: i32,
    pub slug: String
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RealmJson {
    #[serde(skip_deserializing)]
    _links: Option<RealmLinks>,
    pub realms: Vec<Realms>
}

impl RealmJson {
    pub fn new() -> Self {
        if Path::new("realms.json").exists() == false {
            panic!("Could not find realms.json file!")
        }

        let file = File::open("realms.json").unwrap();
        let mut realms: RealmJson = serde_json::from_reader(file).unwrap();
        
        for realm in realms.realms.iter_mut() {
            realm.name = realm.name.replace(" ", "").to_lowercase();
        }
        return realms;
    }
}