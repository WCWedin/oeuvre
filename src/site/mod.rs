use crate::minidom::Element;
use anyhow::{bail, Result};
use std::fs;
use std::path::Path;

mod site;
pub use site::Site;
mod page;
use page::Page;
mod template;
use template::Template;
mod snippet;
use snippet::Snippet;
mod dataset;
use dataset::Dataset;
mod site_config;
pub use site_config::SiteConfig;
mod render;

/// Loads and parses the XML document at the provided path,
/// or else an Err if loading or parsing fail.
fn load_xml(path: &Path) -> Result<Element> {
  let template_text = match fs::read_to_string(&path) {
    Ok(text) => text,
    Err(err) => bail!("{} could not be opened. Cause: {}", path.display(), err),
  };

  match template_text.parse::<Element>() {
    Ok(el) => Ok(el),
    Err(err) => bail!(
      "{} could not be parsed as xml. Cause: {}",
      path.display(),
      err
    ),
  }
}
