use anyhow::{Context, Result};
use config::{Config, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};

#[derive(Serialize, PartialEq, Deserialize, Eq, Debug)]
pub struct ApigConfig {
    pub temporal_service_host: String,
    pub temporal_service_port: String,
    pub environment: String,
}

pub fn init_config_from_env_and_file() -> Result<ApigConfig> {
    Config::builder()
        .add_source(File::new(".env", FileFormat::Ini).required(false))
        .add_source(Environment::default())
        .build()?
        .try_deserialize()
        .with_context(|| "missing required config variables")
}
