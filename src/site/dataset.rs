use super::load_xml;
use crate::minidom::Element;
use crate::PathBuf;
use anyhow::{bail, Result};
use log::{error, info};
use std::collections::HashMap;
use std::path::Path;

/// A set of structure data.
pub struct Dataset {
    pub element: Element,
    pub name: String,
}

impl Dataset {
  fn new(element: Element, datasets: &HashMap<String, Dataset>) -> Result<Dataset> {
    let name = match element.attr("oeuvre-name") {
      None => {
        bail!("Dataset requires a root element with an oeuvre-name attribute");
      }
      Some(attr_value) => attr_value,
    }
    .to_string();

    if datasets.contains_key(&name) {
      bail!(
      "Dataset has the oeuvre-name attribute value {}, which is already in use by another dataset",
      name
    );
    }

    Ok(Dataset { element, name })
  }

  fn load(path: &Path, datasets: &HashMap<String, Dataset>) -> Result<Dataset> {
    let element = load_xml(path)?;
    Dataset::new(element, datasets)
  }

  /// Loads and parses the datasets indicated by `dataset_paths` and returns them
  /// in a HashMap using the dataset name as the key.
  pub fn load_many(dataset_paths: &[PathBuf]) -> HashMap<String, Dataset> {
    let mut datasets = HashMap::<String, Dataset>::new();
    for dataset_path in dataset_paths {
      info!("- Reading {}", dataset_path.display());
      let dataset = match Dataset::load(dataset_path, &datasets) {
        Ok(dataset) => dataset,
        Err(err) => {
          error!("-- {}", err);
          continue;
        }
      };
      info!(
        "-- Loaded dataset {} from {}",
        dataset.name,
        dataset_path.display(),
      );
      datasets.insert(dataset.name.clone(), dataset);
    }
    datasets
  }
  
  /// Loads and parses the data rows indicated by `datarow_paths`
  /// and add each to its dataset in `datasets`.
  pub fn load_rows(datarow_paths: &[PathBuf], datasets: &HashMap<String, Dataset>) {
  }
}
