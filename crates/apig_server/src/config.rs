use anyhow::{Context, Result};
use config::{Config, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};
use toolbox;

#[derive(Serialize, PartialEq, Deserialize, Eq, Debug)]
pub struct ApigConfig {
    #[serde(alias = "TEMPORAL_SERVICE_HOST")]
    pub temporal_service_host: String,
    #[serde(alias = "TEMPORAL_SERVICE_PORT")]
    pub temporal_service_port: String,
    #[serde(alias = "ENVIRONMENT")]
    pub environment: toolbox::Environment,
    #[serde(default = "default_apig_port", alias = "APIG_PORT")]
    pub apig_port: String,
}

pub fn init_config_from_env_and_file() -> Result<ApigConfig> {
    Config::builder()
        .add_source(File::new(".default.env", FileFormat::Ini).required(true))
        .add_source(File::new(".env", FileFormat::Ini).required(false))
        .add_source(Environment::default())
        .build()?
        .try_deserialize()
        .with_context(|| "missing required config variables")
        .into()
}

fn default_apig_port() -> String {
    "3000".to_string()
}
