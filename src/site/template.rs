use super::load_xml;
use crate::minidom::Element;
use crate::PathBuf;
use anyhow::{bail, Result};
use log::{error, info};
use std::collections::HashMap;
use std::path::Path;

/// An element representing an HTML document root with fillable slot elements.
pub struct Template {
  pub element: Element,
  name: String,
}

impl Template {
  fn new(element: Element, templates: &HashMap<String, Template>) -> Result<Template> {
    let name = match element.attr("oeuvre-name") {
      None => {
        bail!("Template requires a root element with an oeuvre-name attribute");
      }
      Some(attr_value) => attr_value,
    }
    .to_string();

    if templates.contains_key(&name) {
      bail!(
      "Template has the oeuvre-name attribute value {}, which is already in use by another template",
      name
    );
    }

    Ok(Template { element, name })
  }

  fn load(template_path: &Path, templates: &HashMap<String, Template>) -> Result<Template> {
    let element = load_xml(template_path)?;
    Template::new(element, templates)
  }

  /// Loads and parses the template indicated by `template_paths` and returns them
  /// in a HashMap using the template name as the key.
  pub fn load_many(template_paths: &[PathBuf]) -> HashMap<String, Template> {
    let mut templates = HashMap::<String, Template>::new();
    for template_path in template_paths {
      info!("- Reading {}", template_path.display());
      let template = match Template::load(template_path, &templates) {
        Ok(template) => template,
        Err(err) => {
          error!("-- {}", err);
          continue;
        }
      };
      info!(
        "-- Loaded template {} from {}",
        template.name,
        template_path.display(),
      );
      templates.insert(template.name.clone(), template);
    }
    templates
  }
}
