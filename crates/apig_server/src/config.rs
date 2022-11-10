use anyhow::{Context, Result};
use config::{Config, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};

#[derive(Serialize, PartialEq, Deserialize, Eq, Debug)]
#[allow(non_snake_case)]
pub struct ApigConfig {
    pub TEMPORAL_SERVICE_HOST: String,
    pub TEMPORAL_SERVICE_PORT: String,
    pub ENVIRONMENT: String,
}

pub fn init_config_from_env_and_file() -> Result<ApigConfig> {
    dbg!(std::env::vars().collect::<Vec<(String, String)>>());
    Config::builder()
        .add_source(File::new(".env", FileFormat::Ini).required(false))
        .add_source(Environment::default())
        .build()?
        .try_deserialize()
        .with_context(|| "missing required config variables")
}
