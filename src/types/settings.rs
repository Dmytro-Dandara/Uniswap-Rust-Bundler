use config::{Config, ConfigError, File};
use serde::Serialize;
use serde_derive::Deserialize;

#[derive(Debug, Serialize, Deserialize)]
#[allow(unused)]
pub struct Connection {
    pub ethereum_rpc_url: String,
    pub flashbots_url: String,
    builders_url: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(unused)]
pub struct Contract {
    pub address: String,
    pub uniswap_v2_router: String,
    pub weth: String,
}

#[derive(Serialize, Deserialize)]
#[allow(unused)]
pub struct Sniper {
    pub private_keys: Vec<String>,
    pub buyback: f64,
    pub max_limit: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(unused)]
pub struct Bundle {
    pub bundler_key: String,
    pub priority_fee: f64,
    pub miner_tip: f64,
    pub retries: u8,
    pub delay_s: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(unused)]
pub struct Settings {
    pub connection: Connection,
    pub contract: Contract,
    pub sniper: Sniper,
    pub bundle: Bundle,
}

impl std::fmt::Debug for Sniper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sniper")
            .field("private_keys", &"<hidden>")
            .field("buyback", &self.buyback)
            .finish()
    }
}

use std::fs::{self, OpenOptions};
use std::io::Write;
impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name("config"))
            .build()?;

        s.try_deserialize()
    }

    pub fn read_config(file_path: &str) -> Result<Settings, Box<dyn std::error::Error>> {
        let file = match fs::read_to_string(file_path) {
            Ok(file) => file,
            Err(_) => {
                eprintln!("File not found: Creating new config file");

                let settings = Settings::new()?;
                let json_settings = serde_json::to_string_pretty(&settings)?;

                let mut file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(file_path)?;
                file.write_all(json_settings.as_bytes())?;

                println!("Created new file with content: {:?}", json_settings);
                json_settings
            }
        };

        let settings: Settings = match serde_json::from_str(&file) {
            Ok(settings) => settings,
            Err(e) => {
                eprintln!("Failed to parse JSON: {:?}", e);
                return Err(Box::new(e));
            }
        };

        Ok(settings)
    }
}
