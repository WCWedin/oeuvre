use super::load_xml;
use crate::minidom::Element;
use crate::PathBuf;
use anyhow::{bail, Result};
use log::{error, info};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use path_clean::PathClean;

use super::Snippet;
use super::Template;
use super::Site;
use super::render::render_template;

/// A single page, as represented by its target template and associated slot values.
/// Each page will render a single XML document to disk.
pub struct Page {
  path: PathBuf,
  template: String,
  slot_values: HashMap<String, Element>,
}

impl Page {
  fn new(element: Element, input_path: &Path) -> Result<Page> {
    let template = match element.attr("oeuvre-template") {
      Some(attr_value) => attr_value.to_string(),
      None => bail!("Page requires a root element with an oeuvre-template attribute"),
    };

    let output_path = match element.attr("oeuvre-path") {
      Some(attr_value) => {
        if attr_value.starts_with('/') { PathBuf::from(attr_value[1..].to_string()) }
        else { input_path.parent().unwrap().join(attr_value) }
      },
      None => input_path.to_path_buf()
    };

    let mut slot_values: HashMap<String, Element> = HashMap::new();
    for child in element.children() {
      let slot_names = child.attr("oeuvre-slot");
      if let Some(slot_names) = slot_names {
        for slot_name in slot_names.to_string().split(',') {
          slot_values.insert(slot_name.trim().to_string(), child.clone());
        }
      }
    }

    Ok(Page {
      path: output_path,
      template,
      slot_values,
    })
  }

  fn load(path: &Path) -> Result<Page> {
    let element = load_xml(path)?;
    Page::new(element, path)
  }

  /// Loads and parses the pages indicated by `page_paths` and returns them
  /// in a HashMap using the relative path to the file as the key.
  pub fn load_many(page_paths: &[PathBuf]) -> HashMap<String, Page> {
    let mut pages = HashMap::<String, Page>::new();
    for page_path in page_paths {
      info!("- Loading page {}", page_path.display());
      let page = match Page::load(page_path) {
        Ok(page) => page,
        Err(err) => {
          error!("-- {}", err);
          continue;
        }
      };
      info!("-- Loaded page {}", page_path.display());
      pages.insert(page.path.display().to_string(), page);
    }
    pages
  }

  fn render(
    &self,
    templates: &HashMap<String, Template>,
    snippets: &HashMap<String, Snippet>,
  ) -> Result<String> {
    let template = match templates.get(&self.template) {
      Some(template) => template,
      None => {
        bail!(
          "Page `{}` requested template `{}`, which does not exist",
          self.path.display(),
          self.template
        );
      }
    };
    let result = render_template(&template.element, &self.slot_values, snippets);
    Ok(String::from(&result))
  }

  fn write(
    &self,
    site: &Site,
  ) -> Result<()> {
    const DOCTYPE_HEADER: &str = "<!DOCTYPE html>\r\n";
    let rendered = match self.render(&site.templates, &site.snippets) {
      Ok(rendered_page) => rendered_page,
      Err(err) => {
        bail!("Failed to render page {}. Cause: {}", &self.path.display(), err);
      }
    };

    let output_path = &site.output_dir.join(&self.path).clean();
    if let Err(err) = fs::create_dir_all(output_path.parent().unwrap()) {
      error!("-- {}", err);
    };

    match fs::write(output_path, format!("{}{}", DOCTYPE_HEADER, rendered)) {
      Ok(_) => Ok(()),
      Err(err) => {
        bail!("Failed to write page {}. Cause: {}", &self.path.display(), err);
      }
    }
  }

  /// Writes all of a site's pages to disk.
  pub fn write_many(site: &Site) {
    for page in site.pages.values() {
      info!("- Writing page {}", &page.path.display());
      if let Err(err) = page.write(site) {
        error!("-- {}", err);
        continue;
      };
      info!("-- Wrote page {}", &page.path.display());
    }
  }
}
