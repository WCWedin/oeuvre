use super::load_xml;
use crate::minidom::Element;
use crate::PathBuf;
use anyhow::{bail, Result};
use log::{error, info};
use std::collections::HashMap;
use std::path::Path;

/// An element containing shared content that can be reused across pages and templates.
pub struct Snippet {
  pub element: Element,
  pub name: String,
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

  /// Loads and parses the snippets indicated by `snippet_paths` and returns them
  /// in a HashMap using the snippet name as the key.
  pub fn load_many(snippet_paths: &[PathBuf]) -> HashMap<String, Snippet> {
    let mut snippets = HashMap::<String, Snippet>::new();
    for snippet_path in snippet_paths {
      info!("- Reading {}", snippet_path.display());
      let snippet = match Snippet::load(snippet_path, &snippets) {
        Ok(snippet) => snippet,
        Err(err) => {
          error!("-- {}", err);
          continue;
        }
      };
      info!(
        "-- Loaded snippet {} from {}",
        snippet.name,
        snippet_path.display(),
      );
      snippets.insert(snippet.name.clone(), snippet);
    }
    snippets
  }
}
