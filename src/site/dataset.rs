use super::load_xml;
use crate::minidom::Element;
use anyhow::{bail, Result};
use log::{error, info};
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

pub enum FieldType {
  String,
  Fragment,
}

pub enum FieldValue {
  String(String),
  Fragment(Element),
}

pub struct Field {
  pub name: String,
  pub required: bool,
  pub field_type: FieldType,
  pub default: Option<FieldValue>,
}

/// A set of structured data.
pub struct Dataset {
    pub name: String,
    pub fields: HashMap<String, Field>,
}

impl Dataset {
  fn new(element: Element, datasets: &HashMap<String, Dataset>) -> Result<Dataset> {
    let name = match element.attr("oeuvre-name") {
      None => {
        bail!("Dataset requires a root element with an oeuvre-name attribute");
      }
      Some(attr_value) => attr_value
    }
    .to_string();

    if datasets.contains_key(&name) {
      bail!(
        "Dataset has the oeuvre-name attribute value {}, which is already in use by another dataset",
        name
      );
    }

    let mut fields = HashMap::<String, Field>::new();

    for child in element.children() {
      let field_name = child.attr("oeuvre-name");
      let field_name = match child.attr("oeuvre-name") {
        None => {
          error!("Data field requires an oeuvre-name attribute");
          continue;
        }
        Some(attr_value) => attr_value.to_string()
      };

      if fields.contains_key(&field_name) {
        error!(
          "Data field has the oeuvre-name attribute value {}, which is already in use by another field",
          name
        );
        continue;
      }

      let required = match child.attr("oeuvre-required") {
        Some(attr_value) => match attr_value.parse() {
          Ok(parsed_value) => parsed_value,
          Err(err) => {
            error!("-- {}", err);
            continue;
          }
        },
        None => false
      };

      let field_type = match child.attr("oeuvre-type") {
        None => {
          error!("Data field requires an oeuvre-type attribute");
          continue;
        }
        Some("string") => FieldType::String,
        Some("fragment") => FieldType::Fragment,
        Some(attr_value) => {
          error!("Data field has invalue oeuvre-type value {}", attr_value);
          continue;
        }
      };
      
      fields.insert(field_name.clone(), Field {
        name: field_name,
        required,
        field_type,
      });
    }

    Ok(Dataset { name, fields })
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
