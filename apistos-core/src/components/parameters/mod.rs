use apistos_models::Schema;

pub mod header;
mod path;
mod query;

const OBJECT_KEYWORDS: [&str; 7] = [
  "properties",
  "required",
  "additionalProperties",
  "maxProperties",
  "minProperties",
  "patternProperties",
  "propertyNames",
];

/// Returns `true` if the schema declares object validation keywords.
fn has_object_validation(schema: &Schema) -> bool {
  OBJECT_KEYWORDS.iter().any(|k| schema.get(*k).is_some())
}

/// Returns the subschemas found in the array keyword `keyword` (e.g. `allOf`) of a schema.
fn subschemas(schema: &Schema, keyword: &str) -> Vec<Schema> {
  schema
    .get(keyword)
    .and_then(|v| v.as_array())
    .map(|arr| arr.iter().filter_map(|v| Schema::try_from(v.clone()).ok()).collect())
    .unwrap_or_default()
}

/// Returns the `properties` of a schema.
fn properties(schema: &Schema) -> Vec<(String, Schema)> {
  schema
    .get("properties")
    .and_then(|v| v.as_object())
    .map(|props| {
      props
        .iter()
        .filter_map(|(name, v)| Schema::try_from(v.clone()).ok().map(|s| (name.clone(), s)))
        .collect()
    })
    .unwrap_or_default()
}
