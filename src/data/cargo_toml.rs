use serde::{Deserialize, Serialize};
use anyhow::Result;
use serde_json::Value;

const TOML: &str = include_str!("../../cargo.toml");

pub fn get_version() -> Result<String> {

    let data: Value = toml::from_str(TOML)?;

    Ok(data["package"]["version"].to_string().replace("\"", ""))
}