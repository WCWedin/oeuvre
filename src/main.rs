use anyhow::{bail, Context, Result};
use glob::glob;
use itertools::Itertools;
use minidom::Element;
use serde_derive::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Config {
  dir: Option<String>,
  output_dir: Option<String>,
  templates: Vec<String>,
  pages: Vec<String>,
}

struct Template {
  element: Element,
  name: String,
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
      None => bail!(
        "-- {} requires a root element with an oeuvre-template attribute",
        path.display()
      ),
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
  let config = read_config_file(&config_path)?;

  println!("Looking for root directory");
  let root_dir = find_root_dir(config_dir, &config.dir.unwrap_or_default())?;

  println!("Using root directory {}", root_dir.display());
  env::set_current_dir(&root_dir).context(format!(
    "Could not change to directory {}",
    root_dir.display()
  ))?;

  println!("Looking for output directory");
  let output_dir = find_output_dir(config_dir, &config.output_dir)?;

  println!("Looking for templates {:?}", config.templates);
  let template_paths = find_files(&config.templates);

  println!("Reading templates");
  let templates = read_template_files(&template_paths);

  println!("Looking for pages {:?}", config.pages);
  let page_paths = find_files(&config.pages);

  println!("Reading pages");
  let pages = read_page_files(&page_paths);

  println!("Writing pages");
  write_pages(&pages, &templates, &output_dir);
  
  Ok(())
}

/// Finds the canonical path to the config file in one of the following places or otherwise returns an `Err`:
/// - `input_path` if `input_path` corresponds to a file
/// - `input_path`/site.toml if `input_path` corresponds to a directory
/// - `"./site.toml"` if `input_path` is `None` and `"./site.toml"` corresponds to a file
fn find_config_file(input_path: &Option<String>) -> Result<PathBuf> {
  const DEFAULT_FILE_NAME: &str = "site.toml";

  let mut path = PathBuf::new();

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
    Ok(path.canonicalize()?)
  } else {
    bail!("{} not found.", path.display())
  }
}

fn read_config_file(path: &Path) -> Result<Config> {
  let config_file =
    fs::read_to_string(&path).context(format!("{} could not be read", path.display()))?;
  toml::from_str::<Config>(&config_file).context(format!(
    "{} could not be parsed as a config file",
    path.display()
  ))
}

fn find_root_dir(start_dir: &Path, dir: &str) -> Result<PathBuf> {
  let dir = start_dir.join(dir);
  let dir = dir
    .canonicalize()
    .context(format!("{} is not a valid path", dir.display()))?;
  if !dir.is_dir() {
    bail!("{} is not a directory.", dir.display());
  }

  Ok(dir)
}

fn find_output_dir(config_dir: &Path, output_dir: &Option<String>) -> Result<PathBuf> {
  const DEFAULT_OUTPUT_DIR: &str = "output";

  let output_dir = match output_dir {
    Some(output_dir) => output_dir,
    None => DEFAULT_OUTPUT_DIR,
  };
  let output_dir = config_dir.join(output_dir);
  let output_dir = output_dir
    .canonicalize()
    .context(format!("{} is not a valid path", output_dir.display()))?;

  if output_dir.is_dir() {
    println!("Output directory found {}", output_dir.display());
    Ok(output_dir)
  } else {
    println!("Creating output directory  {}", output_dir.display());
    match fs::create_dir_all(&output_dir).context(format!(
      "Output directory {} could not be created",
      output_dir.display()
    )) {
      Ok(()) => Ok(output_dir),
      Err(err) => Err(err),
    }
  }
}

fn find_files(glob_patterns: &[String]) -> Vec<PathBuf> {
  glob_patterns
    .iter()
    .flat_map(|pattern| {
      glob(pattern)
        .map_err(|err| println!("{} is not a valid glob pattern. Cause: {}", pattern, err))
        .unwrap()
    })
    .map(|path| {
      path
        .map_err(|err| {
          println!(
            "Globbed path {} could not be read. Cause: {}",
            err.path().display(),
            err
          )
        })
        .unwrap()
    })
    .unique()
    .collect()
}

fn read_template_files(template_paths: &[PathBuf]) -> HashMap<String, Template> {
  let mut templates = HashMap::<String, Template>::new();

  for template_path in template_paths {
    println!("- Reading {}", template_path.display());
    let element = match read_xml_file(template_path) {
      Ok(element) => element,
      Err(err) => {
        println!("-- {}", err);
        break;
      }
    };

    let name = match element.attr("oeuvre-name") {
      None => {
        println!(
          "-- {} requires a root element with an oeuvre-name attribute",
          template_path.display()
        );
        break;
      }
      Some(attr_value) => attr_value,
    }
    .to_string();
    if templates.contains_key(&name) {
      println!(
        "-- {} has the oeuvre-name attribute value {}, which is already in use by another template",
        template_path.display(),
        name
      );
      break;
    }

    let template = Template { element, name };

    println!(
      "- Loaded template {} as {}",
      template_path.display(),
      template.name
    );
    templates.insert(template.name.clone(), template);
  }
  templates
}

fn read_xml_file(path: &Path) -> Result<Element> {
  let template_text =
    fs::read_to_string(&path).context(format!("{} could not be opened", path.display()))?;

  template_text
    .parse::<Element>()
    .context(format!("{} could not be parsed as xml", path.display()))
}

fn read_page_files(page_paths: &[PathBuf]) -> HashMap<String, Page> {
  let mut pages = HashMap::<String, Page>::new();

  for page_path in page_paths {
    println!("- Reading {}", page_path.display());
    let element = match read_xml_file(page_path) {
      Ok(element) => element,
      Err(err) => {
        println!("-- {}", err);
        break;
      }
    };

    let page = match Page::new(element, page_path) {
      Ok(page) => page,
      Err(err) => {
        println!("{}", err);
        break;
      }
    };

    println!("- Loaded page {}", page_path.display());
    pages.insert(page.path.clone(), page);
  }
  pages
}

fn write_pages(pages: &HashMap<String, Page>, templates: &HashMap<String, Template>, output_dir: &Path) {
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
    match fs::write(output_dir.join(&page.path), format!("{}{}", DOCTYPE_HEADER, rendered)) {
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
        "Page `{}` requested template `{}`, which could not be found",
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
      Some(element) => {
        if !element.name().starts_with("oeuvre-") {
          result.append_child(fill_slots(element, slot_values));
        } else if element.name() == "oeuvre-slot" {
          match element.attr("name") {
            None => continue,
            Some(slot_name) => match slot_values.get(slot_name) {
              None => continue,
              Some(slot_value) => {
                if slot_value.name() == "oeuvre-fragment" {
                  for fragment_child in slot_value.nodes() {
                    match fragment_child.as_element() {
                      None => result.append_node(fragment_child.clone()),
                      Some(fragment_child) => {
                        result.append_child(fill_slots(fragment_child, slot_values));
                      }
                    };
                  }
                } else {
                  result.append_child(fill_slots(slot_value, slot_values));
                }
              }
            },
          }
        }
      }
    };
  }
  result
}
