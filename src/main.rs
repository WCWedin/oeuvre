mod minidom;

use anyhow::{bail, Result};
use glob::glob;
use itertools::Itertools;
use log::error;
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
  #[serde(default = "Config::default_snippets")]
  snippets: Vec<String>,
  #[serde(default = "Config::default_pages")]
  pages: Vec<String>,
  #[serde(default = "Config::default_content")]
  content: Vec<String>,
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
  fn default_snippets() -> Vec<String> {
    ["snippets/**/*.html".to_string()].to_vec()
  }
  fn default_pages() -> Vec<String> {
    ["**/*.html".to_string()].to_vec()
  }
  fn default_content() -> Vec<String> {
    ["content/**".to_string()].to_vec()
  }

  fn load(path: &Path) -> Result<Config> {
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
}

struct Template {
  element: Element,
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
}

#[derive(Debug)]
struct Snippet {
  element: Element,
  name: String,
}

impl Snippet {
  fn new(element: Element, snippets: &HashMap<String, Snippet>) -> Result<Snippet> {
    let name = match element.attr("oeuvre-name") {
      None => {
        bail!("Snippet requires a root element with an oeuvre-name attribute");
      }
      Some(attr_value) => attr_value,
    }
    .to_string();

    if snippets.contains_key(&name) {
      bail!(
      "Snippet has the oeuvre-name attribute value {}, which is already in use by another snippet",
      name
    );
    }

    Ok(Snippet { element, name })
  }

  fn load(path: &Path, snippets: &HashMap<String, Snippet>) -> Result<Snippet> {
    let element = load_xml(path)?;
    Snippet::new(element, snippets)
  }
}

struct Page {
  path: String,
  template: String,
  slot_values: HashMap<String, Element>,
}

impl Page {
  fn new(element: Element, path: &Path) -> Result<Page> {
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

  fn load(path: &Path) -> Result<Page> {
    let element = load_xml(path)?;
    Page::new(element, path)
  }

  fn write(
    page: &Page,
    templates: &HashMap<String, Template>,
    snippets: &HashMap<String, Snippet>,
    path: &Path,
  ) -> Result<()> {
    const DOCTYPE_HEADER: &str = "<!DOCTYPE html>\r\n";
    let rendered = match render_page(page, templates, snippets) {
      Ok(rendered_page) => rendered_page,
      Err(err) => {
        bail!("Failed to render page {}. Cause: {}", &page.path, err);
      }
    };
    match fs::write(path, format!("{}{}", DOCTYPE_HEADER, rendered)) {
      Ok(_) => Ok(()),
      Err(err) => {
        bail!("Failed to write page {}. Cause: {}", &page.path, err);
      }
    }
  }
}

fn main() -> Result<()> {
  println!("Looking for config file");
  let config_path = find_config_file(&env::args().nth(1))?;
  let config_dir = config_path.parent().unwrap();

  println!("Reading config file {}", config_path.display());
  let config = Config::load(&config_path)?;

  println!("Looking for root directory");
  let root_dir = find_root_dir(config_dir, &config.dir)?;

  println!("Using root directory {}", root_dir.display());
  use_dir(&root_dir)?;

  // Set up excluded paths collection.
  let mut exclude = Vec::<PathBuf>::new();
  expand_glob(&config.exclude, &mut exclude);

  println!("Looking for output directory");
  let output_dir = create_output_dir(config_dir, &config.output_dir)?;
  let output_glob = format!("{}{}", &config.output_dir, "/**/*");
  expand_glob(&[output_glob], &mut exclude);

  println!("Looking for templates {:?}", config.templates);
  let template_paths: Vec<PathBuf> = expand_glob(&config.templates, &mut exclude);

  println!("Reading templates");
  let templates = load_templates(&template_paths);

  println!("Looking for snippets {:?}", config.snippets);
  let snippet_paths: Vec<PathBuf> = expand_glob(&config.snippets, &mut exclude);

  println!("Reading templates");
  let snippets = load_snippets(&snippet_paths);

  println!("Looking for pages {:?}", config.pages);
  let page_paths = expand_glob(&config.pages, &mut exclude);

  println!("Reading pages");
  let pages = load_pages(&page_paths);

  println!("Looking for content {:?}", config.pages);
  let content_paths = expand_glob(&config.content, &mut exclude);

  println!("Copying content");
  copy_content(&content_paths, &output_dir);

  println!("Writing pages");
  write_pages(&pages, &templates, &snippets, &output_dir);

  Ok(())
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

fn load_templates(template_paths: &[PathBuf]) -> HashMap<String, Template> {
  let mut templates = HashMap::<String, Template>::new();

  for template_path in template_paths {
    println!("- Reading {}", template_path.display());

    let template = match Template::load(template_path, &templates) {
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

fn load_snippets(snippet_paths: &[PathBuf]) -> HashMap<String, Snippet> {
  let mut snippets = HashMap::<String, Snippet>::new();

  for snippet_path in snippet_paths {
    println!("- Reading {}", snippet_path.display());

    let snippet = match Snippet::load(snippet_path, &snippets) {
      Ok(snippet) => snippet,
      Err(err) => {
        println!("-- {}", err);
        continue;
      }
    };

    println!(
      "-- Loaded snippet {} from {}",
      snippet.name,
      snippet_path.display(),
    );
    snippets.insert(snippet.name.clone(), snippet);
  }
  snippets
}

fn load_pages(page_paths: &[PathBuf]) -> HashMap<String, Page> {
  let mut pages = HashMap::<String, Page>::new();

  for page_path in page_paths {
    println!("- Loading page {}", page_path.display());
    let page = match Page::load(page_path) {
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

fn write_pages(
  pages: &HashMap<String, Page>,
  templates: &HashMap<String, Template>,
  snippets: &HashMap<String, Snippet>,
  output_dir: &Path,
) {
  for page in pages.values() {
    println!("- Writing page {}", &page.path);
    if let Err(err) = Page::write(page, templates, snippets, &output_dir.join(&page.path)) {
      println!("-- {}", err);
      continue;
    };
    println!("-- Wrote page {}", &page.path);
  }
}

fn render_page(
  page: &Page,
  templates: &HashMap<String, Template>,
  snippets: &HashMap<String, Snippet>,
) -> Result<String> {
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
  let result = render_template(&template.element, &page.slot_values, snippets);
  Ok(String::from(&result))
}

fn render_template(
  template_element: &Element,
  slot_values: &HashMap<String, Element>,
  snippets: &HashMap<String, Snippet>,
) -> Element {
  let mut result = initialize_element(template_element);

  // Non-element child nodes are emitted unmodified. Oeuvre elements recieve
  // special handling, but all other elements are emitted unmodified.
  for node in template_element.nodes() {
    match node.as_element() {
      None => result.append_node(node.clone()),
      Some(element) => match element.name() {
        "oeuvre-include" => render_include(&element, &mut result, slot_values, snippets),
        "oeuvre-slot" => render_slot(&element, &mut result, slot_values, snippets),
        name if name.starts_with("oeuvre-") => {
          error!("Unknown oeuvre element found: {}", name);
        }
        _ => {
          append_element(element, &mut result, slot_values, snippets);
        }
      },
    };
  }
  result
}

/// Creates a new element, copying the name and attributes of the 
/// template element – though oeuvre attributes will be omitted.
fn initialize_element(template_element: &Element) -> Element {
  let mut result = Element::bare(template_element.name(), template_element.ns());
  for attr in template_element
    .attrs()
    .filter(|attr| !attr.0.starts_with("oeuvre-"))
  {
    result.set_attr(attr.0, attr.1);
  }
  result
}

/// An oeuvre-include element will render the snippet named
/// in its oeuvre-snippet attribute, or its own contents
/// if no such snippet exists. The attribute must be present;
/// otherwise, this function will log an error, render no content
/// for this element, and then continue rendering further elements.
fn render_include(
  element: &Element,
  target: &mut Element,
  slot_values: &HashMap<String, Element>,
  snippets: &HashMap<String, Snippet>,
) {
  match element.attr("oeuvre-snippet") {
    Some(snippet_name) => match snippets.get(snippet_name) {
      Some(snippet) => unwrap_fragment(&snippet.element, target, slot_values, snippets),
      None => unwrap_fragment(&element, target, slot_values, snippets),
    },
    None => {
      error!("Found an oeuvre-include element without a target oeuvre-snippet attribute.")
    }
  }
}

/// An oeuvre-slot element will render the element or fragement
/// provided by the oeuvre-page document being rendered, or its
/// own contents if no such element or fragment exists. The attribute
/// must be present; otherwise, this function will log and error,
/// render no content for this element, and then continue rendering
/// further elements.
fn render_slot(
  element: &Element,
  target: &mut Element,
  slot_values: &HashMap<String, Element>,
  snippets: &HashMap<String, Snippet>,
) {
  match element.attr("oeuvre-name") {
    Some(slot_name) => match slot_values.get(slot_name) {
      Some(slot_value) => match slot_value.name() {
        "oeuvre-fragment" => unwrap_fragment(slot_value, target, slot_values, snippets),
        _ => append_element(slot_value, target, slot_values, snippets),
      },
      None => unwrap_fragment(&element, target, slot_values, snippets),
    },
    None => {
      error!("Found an oeuvre-slot element without an identifying oeuvre-name attribute.")
    }
  }
}

/// Performs template expansion on the provided element and
/// appends the result to `target`.
fn append_element(
  element: &Element,
  target: &mut Element,
  slot_values: &HashMap<String, Element>,
  snippets: &HashMap<String, Snippet>,
) {
  target.append_child(render_template(element, slot_values, snippets));
}

/// Performs template expansion on the children of the provided element and
/// appends the results to `target`. This is used to enable syntax for providing
/// HTML fragments as slot values (oeuvre-fragment) and for appending the
/// fallback content provided by unmatched slots and includes.
fn unwrap_fragment(
  fragment: &Element,
  target: &mut Element,
  slot_values: &HashMap<String, Element>,
  snippets: &HashMap<String, Snippet>,
) {
  for fragment_child in fragment.nodes() {
    match fragment_child.as_element() {
      None => target.append_node(fragment_child.clone()),
      Some(fragment_child) => {
        target.append_child(render_template(fragment_child, slot_values, snippets));
      }
    };
  }
}

/// Copies the site's static content files to the output directory.
fn copy_content(paths: &[PathBuf], output_dir: &Path) {
  for path in paths {
    println!("- Copying file {}", path.display());
    let dir = path.parent().unwrap();
    if let Err(err) = fs::create_dir_all(output_dir.join(dir)) {
      println!("-- {}", err);
      continue;
    };
    if let Err(err) = fs::copy(path, output_dir.join(path)) {
      println!("-- {}", err);
      continue;
    };

    println!("-- Copied file {}", path.display());
  }
}
