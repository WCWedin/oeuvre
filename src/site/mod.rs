use crate::minidom::Element;
use anyhow::{bail, Result};
use std::fs;
use std::path::Path;

use glob::glob;
use itertools::Itertools;
use log::{error, info};
use path_clean::PathClean;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

mod page;
use page::Page;
mod template;
use template::Template;
mod snippet;
use snippet::Snippet;
mod site_config;
pub use site_config::SiteConfig;
mod render;

/// All of the data necessary to render the site to disk, including parsed DOM
/// trees of all document content.
pub struct Site {
  pub pages: HashMap<String, Page>,
  pub templates: HashMap<String, Template>,
  pub snippets: HashMap<String, Snippet>,
  pub content_paths: Vec<PathBuf>,
  pub output_dir: PathBuf,
}

impl Site {
  /// Loads the data specified by `config`,
  /// cataloguing all the required files and loading all document files into memory.
  pub fn load(config: SiteConfig, starting_path: &Path) -> Result<Site> {
    info!("Looking for input directory");
    let input_dir = Site::find_input_dir(starting_path, &config.dir)?;
    info!("Using input directory {}", input_dir.display());
    Site::use_dir(&input_dir)?;

    // Set up excluded paths collection.
    let mut excluded_paths = Vec::<PathBuf>::new();
    // Discarding the returned value; `excluded_paths` will contain the same data anyway.
    Site::expand_glob(&config.exclude, &mut excluded_paths);

    info!("Looking for output directory");
    let output_dir = Site::create_output_dir(starting_path, &config.output_dir)?;
    let output_glob = format!("{}{}", &config.output_dir, "/**/*");
    // Discarding the returned value; we only need to add the output paths to `excluded_paths`.
    Site::expand_glob(&[output_glob], &mut excluded_paths);

    info!("Looking for templates {:?}", config.templates);
    let template_paths: Vec<PathBuf> = Site::expand_glob(&config.templates, &mut excluded_paths);
    info!("Reading templates");
    let templates = Template::load_many(&template_paths);

    info!("Looking for snippets {:?}", config.snippets);
    let snippet_paths: Vec<PathBuf> = Site::expand_glob(&config.snippets, &mut excluded_paths);
    info!("Reading templates");
    let snippets = Snippet::load_many(&snippet_paths);

    info!("Looking for pages {:?}", config.pages);
    let page_paths = Site::expand_glob(&config.pages, &mut excluded_paths);
    info!("Reading pages");
    let pages = Page::load_many(&page_paths);

    info!("Looking for assets {:?}", config.pages);
    let content_paths = Site::expand_glob(&config.assets, &mut excluded_paths);

    Ok(Site {
      pages,
      templates,
      snippets,
      content_paths,
      output_dir,
    })
  }

  /// Renders the site and writes the output to disk.
  pub fn render(&self) -> Result<()> {
    info!("Copying assets");
    self.copy_assets();
    info!("Writing pages");
    Page::write_many(self);

    Ok(())
  }

  /// Expands all glob patterns into file paths and returns the result.
  /// Paths listed in `excluded_paths` will be ignored, and the result of the
  /// expansion will be appended to the `excluded_paths` list.
  fn expand_glob(glob_patterns: &[String], excluded_paths: &mut Vec<PathBuf>) -> Vec<PathBuf> {
    let found_paths = glob_patterns
      .iter()
      .filter_map(|pattern| match glob(pattern) {
        Ok(paths) => Some(
          paths
            .filter_map(move |item| match item {
              Ok(val) => Some(val),
              Err(err) => {
                error!("Globbed path could not be read. Cause: {}", err);
                None
              }
            })
            .collect::<Vec<PathBuf>>(),
        ),
        Err(err) => {
          error!("{} is not a valid glob pattern. Cause: {}", pattern, err);
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

  fn find_input_dir(start_dir: &Path, dir: &str) -> Result<PathBuf> {
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
      info!("Output directory found {}", output_dir.display());
      Ok(output_dir)
    } else {
      info!("Creating output directory {}", output_dir.display());
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

  /// Copies the site's static assets files to the output directory.
  fn copy_assets(&self) {
    for path in &self.content_paths {
      info!("- Copying file {}", path.display());
      let dir = path.parent().unwrap();
      if let Err(err) = fs::create_dir_all(self.output_dir.join(dir)) {
        error!("-- {}", err);
        continue;
      };
      if let Err(err) = fs::copy(path, self.output_dir.join(path)) {
        error!("-- {}", err);
        continue;
      };

      info!("-- Copied file {}", path.display());
    }
  }
}

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
