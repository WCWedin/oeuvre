mod minidom;

use anyhow::{bail, Result};
use glob::glob;
use itertools::Itertools;
use minidom::Element;
use path_clean::PathClean;
use serde_derive::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Config {
  #[serde(default = "Config::default_dir")]
  dir: String,
  #[serde(default = "Config::default_output_dir")]
  output_dir: String,
  #[serde(default = "Config::default_exclude")]
  exclude: Vec<String>,
  #[serde(default = "Config::default_templates")]
  templates: Vec<String>,
  #[serde(default = "Config::default_pages")]
  pages: Vec<String>,
}

impl Config {
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
    ["templates/**/*.html".to_string()].to_vec()
  }
  fn default_pages() -> Vec<String> {
    ["**/*.html".to_string()].to_vec()
  }
}

struct Template {
  element: Element,
  name: String,
}

impl Template {
  pub fn new(element: Element, templates: &HashMap<String, Template>) -> Result<Template> {
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
}

struct Page {
  path: String,
  template: String,
  slot_values: HashMap<String, Element>,
}

impl Page {
  pub fn new(element: Element, path: &Path) -> Result<Page> {
    let template = match element.attr("oeuvre-template") {
      Some(attr_value) => attr_value,
      None => bail!("Page requires a root element with an oeuvre-template attribute",),
    }
    .to_string();

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
      path: path.display().to_string(),
      template,
      slot_values,
    })
  }
}

fn main() -> Result<()> {
  println!("Looking for config file");
  let config_path = find_config_file(&env::args().nth(1))?;
  let config_dir = config_path.parent().unwrap();

  println!("Reading config file {}", config_path.display());
  let config = load_config(&config_path)?;

  println!("Looking for root directory");
  let root_dir = find_root_dir(config_dir, &config.dir)?;

  println!("Using root directory {}", root_dir.display());
  use_dir(&root_dir)?;

  println!("Looking for output directory");
  let output_dir = create_output_dir(config_dir, &config.output_dir)?;

  // Set up excluded paths collection.
  let mut exclude = Vec::<PathBuf>::new();
  exclude = expand_glob(&config.exclude, &mut exclude);
  let ouput_glob = format!("{}{}", &config.output_dir, "/**/*");
  let mut output_files = expand_glob(&[ouput_glob], &mut exclude);
  exclude.append(&mut output_files);
  exclude.sort();

  println!("Looking for templates {:?}", config.templates);
  let template_paths: Vec<PathBuf> = expand_glob(&config.templates, &mut exclude);
  exclude.append(&mut template_paths.clone());

  println!("Reading templates");
  let templates = load_templates(&template_paths);

  println!("Looking for pages {:?}", config.pages);
  let page_paths = expand_glob(&config.pages, &mut exclude);

  println!("Reading pages");
  let pages = load_pages(&page_paths);

  println!("Writing pages");
  write_pages(&pages, &templates, &output_dir);

  Ok(())
}

/// Finds the root path to the config file in one of the following places or otherwise returns an `Err`:
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

fn load_config(path: &Path) -> Result<Config> {
  let config_file = match fs::read_to_string(&path) {
    Ok(text) => text,
    Err(err) => bail!("{} could not be opened. Cause: {}", path.display(), err),
  };

  match toml::from_str::<Config>(&config_file) {
    Ok(el) => Ok(el),
    Err(err) => bail!(
      "{} could not be parsed as a config file. Cause: {}",
      path.display(),
      err
    ),
  }
}

fn find_root_dir(start_dir: &Path, dir: &str) -> Result<PathBuf> {
  let dir = start_dir.join(dir);
  let dir = dir.clean();
  if !dir.is_dir() {
    bail!("{} is not a directory.", dir.display());
  }

  Ok(dir)
}

fn use_dir(dir: &Path) -> Result<()> {
  match env::set_current_dir(&dir) {
    Ok(_) => Ok(()),
    Err(_) => {
      bail!("Could not change to directory {}", dir.display())
    }
  }
}

fn create_output_dir(config_dir: &Path, output_dir: &str) -> Result<PathBuf> {
  let output_dir = config_dir.join(output_dir).clean();

  if output_dir.is_dir() {
    println!("Output directory found {}", output_dir.display());
    Ok(output_dir)
  } else {
    println!("Creating output directory  {}", output_dir.display());
    match fs::create_dir_all(&output_dir) {
      Err(err) => bail!(
        "Output directory {} could not be created. Cause: {}",
        output_dir.display(),
        err
      ),
      Ok(_) => Ok(output_dir),
    }
  }
}

fn expand_glob(glob_patterns: &[String], excluded_paths: &mut Vec<PathBuf>) -> Vec<PathBuf> {
  let found_paths = glob_patterns
    .iter()
    .filter_map(|pattern| match glob(pattern) {
      Ok(paths) => Some(
        paths
          .filter_map(move |item| match item {
            Ok(val) => Some(val),
            Err(err) => {
              println!("Globbed path could not be read. Cause: {}", err);
              None
            }
          })
          .collect::<Vec<PathBuf>>(),
      ),
      Err(err) => {
        println!("{} is not a valid glob pattern. Cause: {}", pattern, err);
        None
      }
    })
    .flatten()
    .unique()
    .sorted();

  // Remove excluded paths from the results.
  let mut excluded_paths_iter = excluded_paths.iter();
  let mut excluded_path = excluded_paths_iter.next();
  let found_paths: Vec<PathBuf> = found_paths
    .filter(|path| loop {
      if excluded_path.is_none() || path < excluded_path.unwrap() {
        break true;
      } else if path == excluded_path.unwrap() {
        excluded_path = excluded_paths_iter.next();
        break false;
      } else {
        excluded_path = excluded_paths_iter.next();
      }
    })
    .collect();

  // Add the results to the excluded set to prevent them from being processed again.
  excluded_paths.append(&mut found_paths.clone());
  excluded_paths.sort();
  found_paths
}

fn load_templates(template_paths: &[PathBuf]) -> HashMap<String, Template> {
  let mut templates = HashMap::<String, Template>::new();

  for template_path in template_paths {
    println!("- Reading {}", template_path.display());

    let template = match load_template(template_path, &templates) {
      Ok(template) => template,
      Err(err) => {
        println!("-- {}", err);
        continue;
      }
    };

    println!(
      "-- Loaded template {} from {}",
      template.name,
      template_path.display(),
    );
    templates.insert(template.name.clone(), template);
  }
  templates
}

fn load_template(template_path: &Path, templates: &HashMap<String, Template>) -> Result<Template> {
  let element = load_xml(template_path)?;
  Template::new(element, templates)
}

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

fn load_pages(page_paths: &[PathBuf]) -> HashMap<String, Page> {
  let mut pages = HashMap::<String, Page>::new();

  for page_path in page_paths {
    println!("- Loading page {}", page_path.display());
    let page = match load_page(page_path) {
      Ok(page) => page,
      Err(err) => {
        println!("-- {}", err);
        continue;
      }
    };

    println!("-- Loaded page {}", page_path.display());
    pages.insert(page.path.clone(), page);
  }
  pages
}

fn load_page(page_path: &Path) -> Result<Page> {
  let element = load_xml(page_path)?;
  Page::new(element, page_path)
}

fn write_pages(
  pages: &HashMap<String, Page>,
  templates: &HashMap<String, Template>,
  output_dir: &Path,
) {
  const DOCTYPE_HEADER: &str = "<!DOCTYPE html>\r\n";

  for page in pages.values() {
    println!("Writing page {}", &page.path);
    let rendered = match render_page(page, templates) {
      Ok(rendered_page) => rendered_page,
      Err(err) => {
        println!("Failed to render page {}. Cause: {}", &page.path, err);
        continue;
      }
    };
    match fs::write(
      output_dir.join(&page.path),
      format!("{}{}", DOCTYPE_HEADER, rendered),
    ) {
      Ok(()) => (),
      Err(err) => {
        println!("Failed to write page {}. Cause: {}", &page.path, err);
        continue;
      }
    }
  }
}

fn render_page(page: &Page, templates: &HashMap<String, Template>) -> Result<String> {
  let template = match templates.get(&page.template) {
    Some(template) => template,
    None => {
      bail!(
        "Page `{}` requested template `{}`, which does not exist",
        page.path,
        page.template
      );
    }
  };
  let result = fill_slots(&template.element, &page.slot_values);
  Ok(String::from(&result))
}

fn fill_slots(template: &Element, slot_values: &HashMap<String, Element>) -> Element {
  let mut result = Element::bare(template.name(), template.ns());
  for attr in template
    .attrs()
    .filter(|attr| !attr.0.starts_with("oeuvre-"))
  {
    result.set_attr(attr.0, attr.1);
  }
  for node in template.nodes() {
    match node.as_element() {
      None => result.append_node(node.clone()),
      Some(element) => match element.name() {
        "oeuvre-slot" => match element.attr("name") {
          None => continue,
          Some(slot_name) => match slot_values.get(slot_name) {
            None => continue,
            Some(slot_value) => match slot_value.name() {
              "oeuvre-fragment" => {
                for fragment_child in slot_value.nodes() {
                  match fragment_child.as_element() {
                    None => result.append_node(fragment_child.clone()),
                    Some(fragment_child) => {
                      result.append_child(fill_slots(fragment_child, slot_values));
                    }
                  };
                }
              }
              _ => {
                result.append_child(fill_slots(slot_value, slot_values));
              }
            },
          },
        },
        name if name.starts_with("oeuvre-") => {}
        _ => {
          result.append_child(fill_slots(element, slot_values));
        }
      },
    };
  }
  result
}
