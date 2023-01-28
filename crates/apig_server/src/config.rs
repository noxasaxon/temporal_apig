use anyhow::{Context, Result};
use config::{Config, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};

#[derive(Serialize, PartialEq, Eq, Deserialize, Debug)]
#[allow(non_camel_case_types)]
pub enum Environments {
    local,
    dev,
    stage,
    prod,
}

#[derive(Serialize, PartialEq, Deserialize, Eq, Debug)]
pub struct ApigConfig {
    pub temporal_service_host: String,
    pub temporal_service_port: String,
    pub environment: Environments,
    pub apig_port: String,
}

pub fn init_config_from_env_and_file() -> Result<ApigConfig> {
    Config::builder()
        .set_default("apig_port", "3000".to_string())
        .unwrap()
        .add_source(File::new(".default.env", FileFormat::Ini).required(true))
        .add_source(File::new(".env", FileFormat::Ini).required(false))
        .add_source(Environment::default())
        .build()?
        .try_deserialize()
        .with_context(|| "missing required config variables")
}
