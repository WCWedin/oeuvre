

use anyhow::{bail, Result};
use serde_derive::Deserialize;
use std::fs;
use std::path::Path;

/// A configuration object used for deserlializing corresponding toml config files.
#[derive(Deserialize)]
pub struct SiteConfig {
  #[serde(default = "SiteConfig::default_dir")]
  pub dir: String,
  #[serde(default = "SiteConfig::default_output_dir")]
  pub output_dir: String,
  #[serde(default = "SiteConfig::default_exclude")]
  pub exclude: Vec<String>,
  #[serde(default = "SiteConfig::default_templates")]
  pub templates: Vec<String>,
  #[serde(default = "SiteConfig::default_snippets")]
  pub snippets: Vec<String>,
  #[serde(default = "SiteConfig::default_datasets")]
  pub datasets: Vec<String>,
  #[serde(default = "SiteConfig::default_datarows")]
  pub datarows: Vec<String>,
  #[serde(default = "SiteConfig::default_assets")]
  pub assets: Vec<String>,
  #[serde(default = "SiteConfig::default_pages")]
  pub pages: Vec<String>,
}

impl SiteConfig {
  fn default_dir() -> String {
    "./".to_string()
  }
  fn default_output_dir() -> String {
    "output/".to_string()
  }
  fn default_exclude() -> Vec<String> {
    Vec::new()
  }
  fn default_templates() -> Vec<String> {
    ["templates/**/*.xml".to_string()].to_vec()
  }
  fn default_snippets() -> Vec<String> {
    ["snippets/**/*.xml".to_string()].to_vec()
  }
  fn default_datasets() -> Vec<String> {
    ["data/*.xml".to_string()].to_vec()
  }
  fn default_datarows() -> Vec<String> {
    ["data/*/**/*.xml".to_string()].to_vec()
  }
  fn default_assets() -> Vec<String> {
    ["assets/**".to_string()].to_vec()
  }
  fn default_pages() -> Vec<String> {
    ["**/*.xml".to_string()].to_vec()
  }

  /// Deserializes the toml file at the given path into a SiteConfig,
  /// or returns an error if the file could not be read and parsed.
  pub fn load(path: &Path) -> Result<SiteConfig> {
    let config_file = match fs::read_to_string(&path) {
      Ok(text) => text,
      Err(err) => bail!("{} could not be opened. Cause: {}", path.display(), err),
    };
    match toml::from_str::<SiteConfig>(&config_file) {
      Ok(el) => Ok(el),
      Err(err) => bail!(
        "{} could not be parsed as a config file. Cause: {}",
        path.display(),
        err
      ),
    }
  }
}
