mod minidom;
mod site;

use crate::site::{Site, SiteConfig};
use anyhow::{bail, Result};
use log::info;
use path_clean::PathClean;
use simple_logger::SimpleLogger;
use std::env;
use std::path::Path;
use std::path::PathBuf;

fn main() -> Result<()> {
  SimpleLogger::new().init().unwrap();
  info!("Looking for config file");
  let config_path = find_config_file(&env::args().nth(1))?;
  let config_dir = config_path.parent().unwrap();
  info!("Reading config file {}", config_path.display());
  let config = SiteConfig::load(&config_path)?;

  let site = Site::load(config, config_dir)?;
  site.render()
}

/// Finds the root path to the config file in one of the following places,
/// or otherwise returns an `Err`:
/// - `input_path` if `input_path` corresponds to a file
/// - `input_path`/site.toml if `input_path` corresponds to a directory
/// - `"./site.toml"` if `input_path` is `None` and `"./site.toml"` corresponds to a file
fn find_config_file(input_path: &Option<String>) -> Result<PathBuf> {
  const DEFAULT_FILE_NAME: &str = "site.toml";

  let mut path = PathBuf::new();
  path.push(env::current_dir()?);

  match input_path {
    None => path.push(DEFAULT_FILE_NAME),
    Some(input_path) => {
      path.push(input_path);
      if !Path::new(input_path).is_file() {
        path.push(DEFAULT_FILE_NAME);
      }
    }
  };

  if path.is_file() {
    Ok(path.clean())
  } else {
    bail!("{} not found.", path.display())
  }
}
