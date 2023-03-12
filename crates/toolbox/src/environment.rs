use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::EnumString;

pub const SECRET_PREFIX: &str = "SECRET_";
pub const ENVIRONMENT_STR: &str = "ENVIRONMENT";

#[derive(EnumString, PartialEq, Debug, Serialize, Deserialize, Eq)]
pub enum Environment {
    // duplicated alias/renaming due to config-rs quirks
    #[serde(alias = "LOCAL", rename = "local")]
    LOCAL,
    #[serde(alias = "DEV", rename = "dev")]
    DEV,
    #[serde(alias = "STAGE", rename = "stage")]
    STAGE,
    #[serde(alias = "PROD", rename = "prod")]
    PROD,
}

///  Get file path from env var & return file contents as a string.
pub fn read_file_from_env_path(env_secret_name: &str) -> Result<String> {
    let file_name = std::env::var(env_secret_name)?;
    std::fs::read_to_string(&file_name).with_context(|| {
        format!(
            "failed to read file_path {} found at env var {}",
            file_name, env_secret_name
        )
    })
}

/// Read env var `ENVIRONMNENT`, raises an error if not set or not valid environment
pub fn get_deployment_env() -> Result<Environment> {
    let env = std::env::var(ENVIRONMENT_STR)?.to_uppercase();
    Environment::from_str(&env)
        .with_context(|| format!("`ENVIRONMENT` variable is not a valid environment: {}", env))
}

pub fn get_env_var(env_var_name: &str) -> Result<String> {
    std::env::var(env_var_name)
        .with_context(|| format!("Could not read environment variable: `{}`", env_var_name))
}

pub fn get_envoy_host(role: &str) -> Result<String> {
    get_env_var(format!("{}_SERVICE_HOST", role.to_uppercase().replace('-', "_")).as_str())
}

pub fn get_envoy_port(role: &str) -> Result<String> {
    get_env_var(format!("{}_SERVICE_PORT", role.to_uppercase().replace('-', "_")).as_str())
}

#[cfg(test)]
mod test {
    use crate::*;
    use serde::Deserialize;
    use serde_json::json;

    #[test]
    fn test_environment_string_to_enum() {
        // lowercase
        std::env::set_var(ENVIRONMENT_STR, "local");
        let env_enum = get_deployment_env().unwrap();
        assert_eq!(env_enum, Environment::LOCAL);

        // uppercase
        std::env::set_var(ENVIRONMENT_STR, "LOCAL");
        let env_enum = get_deployment_env().unwrap();
        assert_eq!(env_enum, Environment::LOCAL);
    }

    #[test]
    fn test_environment_from_lowercase() {
        #[derive(Deserialize)]
        struct EnvWrapper {
            environment: Environment,
        }

        let _env_wrapper: EnvWrapper = serde_json::from_value(json!({
            "environment" : "local"
        }))
        .unwrap();

        let _env_wrapper: EnvWrapper = serde_json::from_value(json!({
            "environment" : "LOCAL"
        }))
        .unwrap();
    }
}
