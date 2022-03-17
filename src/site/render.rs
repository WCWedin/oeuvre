use crate::site::Snippet;
use crate::minidom::Element;
use log::error;
use std::collections::HashMap;

/// Injects slot values and snippet content into a template,
/// performing template expansion on all child elements.
pub fn render_template(
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
        "oeuvre-include" => render_include(element, &mut result, slot_values, snippets),
        "oeuvre-slot" => render_slot(element, &mut result, slot_values, snippets),
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

/// An oeuvre-include element will render the snippet named
/// in its oeuvre-snippet attribute, or its own contents
/// if no such snippet exists. The attribute must be present;
/// otherwise, this function will log an error and render no
/// content for this element.
fn render_include(
  element: &Element,
  target: &mut Element,
  slot_values: &HashMap<String, Element>,
  snippets: &HashMap<String, Snippet>,
) {
  match element.attr("oeuvre-snippet") {
    Some(snippet_name) => match snippets.get(snippet_name) {
      Some(snippet) => unwrap_fragment(&snippet.element, target, slot_values, snippets),
      None => unwrap_fragment(element, target, slot_values, snippets),
    },
    None => {
      error!("Found an oeuvre-include element without a target oeuvre-snippet attribute.")
    }
  }
}

/// An oeuvre-slot element will render the element or fragement
/// provided by the oeuvre-page document being rendered, or its
/// own contents if no such element or fragment exists. The attribute
/// must be present; otherwise, this function will log an error and
/// render no content for this element.
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
      None => unwrap_fragment(element, target, slot_values, snippets),
    },
    None => {
      error!("Found an oeuvre-slot element without an identifying oeuvre-name attribute.")
    }
  }
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
