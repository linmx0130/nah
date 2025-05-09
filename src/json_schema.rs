/**
 * Utilities to process JSON Schema.
 */
use serde_json::Value;

use crate::types::NahError;

enum JSONSchemaTypeName {
  String,
  Object,
  Number,
  Boolean,
  Null,
}

/**
 * Create a JSON instance template from a JSON Schema.
 */
pub fn create_instance_template(schema: &Value) -> Result<String, NahError> {
  let mut lines = create_instance_template_impl(schema, 0)?;
  lines.last_mut().unwrap().pop();
  Ok(lines.join("\n"))
}

fn create_instance_template_impl(schema: &Value, indent: usize) -> Result<Vec<String>, NahError> {
  let type_name = get_type_name(schema)?;
  let mut indent_string = String::with_capacity(indent);
  for _i in 0..indent {
    indent_string.push(' ');
  }

  match type_name {
    JSONSchemaTypeName::String => Ok(vec!["\"<FILL A STRING>\",".to_owned()]),
    JSONSchemaTypeName::Null => Ok(vec!["null,".to_owned()]),
    JSONSchemaTypeName::Number => Ok(vec!["<FILL A NUMBER>,".to_owned()]),
    JSONSchemaTypeName::Boolean => Ok(vec!["<FILL A BOOLEAN VALUE>,".to_owned()]),
    JSONSchemaTypeName::Object => {
      let mut result = Vec::new();
      let properties = get_properties(schema)?;
      result.push("{".to_owned());
      for (field_name, field_schema) in properties.iter() {
        let field_template = create_instance_template_impl(field_schema, indent + 4)?;
        // grab the first line to connect field_name;
        result.push(format!(
          "{}    \"{}\": {}",
          indent_string, field_name, field_template[0]
        ));
        for i in 1..field_template.len() {
          result.push(field_template[i].clone());
        }
      }
      // Remove the extra comma in the end of last line to follow JSON format
      result.last_mut().and_then(|line| {
        if line.ends_with(",") {
          line.pop();
        };
        Some(line)
      });
      result.push(format!("{}}},", indent_string));
      Ok(result)
    }
  }
}

/// Return true if the argument is an empty JSON object.
pub fn is_empty_object(value: &Value) -> bool {
  value
    .as_object()
    .and_then(|obj| if obj.len() == 0 { Some(()) } else { None })
    .is_some()
}

/**
 * Get type of a JSON Schema
 */
fn get_type_name(schema: &Value) -> Result<JSONSchemaTypeName, NahError> {
  match schema
    .as_object()
    .and_then(|v| v.get("type"))
    .and_then(|v| v.as_str())
  {
    Some(type_name) => match type_name {
      "string" => Ok(JSONSchemaTypeName::String),
      "object" => Ok(JSONSchemaTypeName::Object),
      "number" => Ok(JSONSchemaTypeName::Number),
      "boolean" => Ok(JSONSchemaTypeName::Boolean),
      "null" => Ok(JSONSchemaTypeName::Null),
      _ => Err(NahError::received_invalid_json_schema(&format!(
        "unknown type name : {}",
        type_name
      ))),
    },
    None => Err(NahError::received_invalid_json_schema(
      "type name not found",
    )),
  }
}

/**
 * Get properties as a list of field name and the type schema
 */
fn get_properties(schema: &Value) -> Result<Vec<(&String, &Value)>, NahError> {
  let properties = match schema
    .as_object()
    .and_then(|v| v.get("properties"))
    .and_then(|v| v.as_object())
  {
    None => {
      return Err(NahError::received_invalid_json_schema(
        "properties not found on an object",
      ));
    }
    Some(p) => p,
  };
  Ok(properties.iter().collect())
}

#[cfg(test)]
mod tests {
  use crate::json_schema::*;
  use serde_json::*;

  #[test]
  fn test_is_empty_object() {
    assert_eq!(is_empty_object(&json!({})), true);
    assert_eq!(is_empty_object(&json!({"key": 1})), false);
    assert_eq!(is_empty_object(&json!(0)), false);
  }
}
