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
  templates: Vec<String>,
  pages: Vec<String>,
}

struct Template {
  element: Element,
  name: String,
  //slots: Vec<String>
}

struct Page {
  element: Element,
  path: String,
  template: String,
  //slots: Vec<String>
}

fn main() -> Result<()> {
  println!("Looking for config file");
  let config_path = find_config_file(&env::args().nth(1))?;

  println!("Reading config file {}", config_path.display());
  let config = read_config_file(&config_path)?;

  println!("Looking for root directory");
  let dir = find_root_dir(
    config_path.parent().unwrap(),
    &config.dir.unwrap_or_default(),
  )?;

  println!("Using directory {}", dir.display());
  env::set_current_dir(&dir).unwrap();

  println!("Looking for templates {:?}", config.templates);
  let template_paths = find_files(&config.templates);

  println!("Reading templates");
  let templates = read_template_files(&template_paths);

  println!("Looking for pages {:?}", config.pages);
  let page_paths = find_files(&config.pages);

  let pages = read_page_files(&page_paths);

  Ok(())
}

/// Finds the canonical path to the config file in one of the following places or otherwise returns an `Err`:
/// - `input_path` if `input_path` corresponds to a file
/// - `input_path`/site.toml if `input_path` corresponds to a directory
/// - ./site.toml if `input_path` is `None` and a file named site.toml exists in the current directory
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
  let config_file = match fs::read_to_string(&path) {
    Ok(path) => path,
    Err(err) => bail!("{} could not be read. Cause: {}", path.display(), err),
  };
  match toml::from_str::<Config>(&config_file) {
    Ok(config_file) => Ok(config_file),
    Err(err) => bail!(
      "{} could not be parsed as a config file. Cause: {}",
      path.display(),
      err
    ),
  }
}

fn find_root_dir(start_dir: &Path, dir: &str) -> Result<PathBuf> {
  let dir = start_dir.join(dir);
  let dir = dir
    .canonicalize()
    .context(format!("{} is not a valid path.", dir.display()))?;
  if !dir.is_dir() {
    bail!("{} is not a directory.", dir.display());
  }

  Ok(dir)
}

fn find_files(glob_patterns: &[String]) -> Vec<PathBuf> {
  glob_patterns
    .iter()
    .flat_map(|pattern| {
      glob(pattern)
        .map_err(|_| println!("{} is not a valid glob pattern", pattern))
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

    let name = match element.attrs().find(|attr| attr.0 == "oeuvre-name") {
      None => {
        println!(
          "-- {} requires a root element with an oeuvre-name attribute",
          template_path.display()
        );
        break;
      }
      Some((_, attr_value)) => attr_value,
    }.to_string();
    if templates.contains_key(&name) {
      println!(
        "-- {} has the oeuvre-name attribute value {}, which is already in use by another template",
        template_path.display(),
        name
      );
      break;
    }

    let template = Template {
      element,
      name,
    };

    println!("- Loaded template {} as {}", template_path.display(), template.name);
    templates.insert(template.name.clone(), template);
  }
  templates
}

fn read_xml_file(path: &Path) -> Result<Element> {
  let template_text = match fs::read_to_string(&path) {
    Ok(template_text) => template_text,
    Err(err) => {
      bail!("{} could not be opened. Cause: {}", path.display(), err);
    }
  };

  match template_text.parse::<Element>() {
    Ok(template) => Ok(template),
    Err(err) => {
      bail!(
        "{} could not be parsed as xml. Cause: {}",
        path.display(),
        err
      );
    }
  }
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

    let template = match element.attrs().find(|attr| attr.0 == "oeuvre-template") {
      None => {
        println!(
          "-- {} requires a root element with an oeuvre-template attribute",
          page_path.display()
        );
        break;
      }
      Some((_, attr_value)) => attr_value,
    }.to_string();

    let page = Page {
      element,
      path: page_path.display().to_string(),
      template
    };

    println!("- Loaded page {}", page_path.display());
    pages.insert(page.path.clone(), page);
  }
  pages
}
