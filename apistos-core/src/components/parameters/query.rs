use crate::ApiComponent;
#[cfg(feature = "query")]
use actix_web::web::Query;
#[cfg(feature = "lab_query")]
use actix_web_lab::extract::Query as LabQuery;
use apistos_models::Schema;
use apistos_models::paths::ParameterStyle;
use apistos_models::paths::{Parameter, ParameterDefinition, ParameterIn, RequestBody};
use apistos_models::reference_or::ReferenceOr;
#[cfg(all(feature = "lab_query", feature = "garde"))]
use garde_actix_web::web::LabQuery as GardeLabQuery;
#[cfg(all(feature = "qs_query", feature = "garde"))]
use garde_actix_web::web::QsQuery as GardeQsQuery;
#[cfg(all(feature = "query", feature = "garde"))]
use garde_actix_web::web::Query as GardeQuery;
use schemars::json_schema;
#[cfg(feature = "qs_query")]
use serde_qs::actix::QsQuery;
use std::collections::HashMap;

use super::{has_object_validation, properties, subschemas};

#[allow(unused_macro_rules)]
macro_rules! impl_query {
  ($ty:ident) => {
    impl_query!($ty, hashmap_style: None, style: None, explode: None);
  };
  ($ty:ident, hashmap_style: $hashmap_style:expr) => {
    impl_query!($ty, hashmap_style: $hashmap_style, style: None, explode: None);
  };
  ($ty:ident, style: $style:expr, explode: $explode:expr) => {
    impl_query!($ty, hashmap_style: None, style: $style, explode: $explode);
  };
  ($ty:ident, hashmap_style: $hashmap_style:expr, style: $style:expr, explode: $explode:expr) => {
    impl<T> ApiComponent for $ty<T>
    where
      T: ApiComponent,
    {
      fn required() -> bool {
        T::required()
      }

      fn child_schemas() -> Vec<(String, ReferenceOr<Schema>)> {
        T::child_schemas()
      }

      fn raw_schema() -> Option<ReferenceOr<Schema>> {
        T::raw_schema()
      }

      fn schema() -> Option<(String, ReferenceOr<Schema>)> {
        None
      }

      fn request_body() -> Option<RequestBody> {
        None
      }

      fn parameters() -> Vec<Parameter> {
        let schema = T::schema().map(|(_, sch)| sch).or_else(Self::raw_schema);
        parameters_from_schema(schema, None, &None, &$style, $explode)
      }
    }

    impl<K, V> ApiComponent for $ty<HashMap<K, V>>
    where
      V: ApiComponent,
    {
      fn required() -> bool {
        false
      }

      fn child_schemas() -> Vec<(String, ReferenceOr<Schema>)> {
        V::child_schemas()
      }

      fn raw_schema() -> Option<ReferenceOr<Schema>> {
        V::raw_schema()
      }

      fn schema() -> Option<(String, ReferenceOr<Schema>)> {
        None
      }

      fn request_body() -> Option<RequestBody> {
        None
      }

      fn parameters() -> Vec<Parameter> {
        let schema = V::schema().map(|(_, sch)| sch).or_else(Self::raw_schema);
        parameters_from_hashmap(schema, $hashmap_style)
      }
    }
  };
}

#[cfg(feature = "query")]
impl_query!(Query);
#[cfg(feature = "lab_query")]
impl_query!(LabQuery, style: Some(ParameterStyle::Form), explode: Some(true));
#[cfg(feature = "qs_query")]
impl_query!(QsQuery, hashmap_style: Some(ParameterStyle::DeepObject));
#[cfg(all(feature = "query", feature = "garde"))]
impl_query!(GardeQuery);
#[cfg(all(feature = "qs_query", feature = "garde"))]
impl_query!(GardeQsQuery, hashmap_style: Some(ParameterStyle::DeepObject));
#[cfg(all(feature = "lab_query", feature = "garde"))]
impl_query!(GardeLabQuery, style: Some(ParameterStyle::Form), explode: Some(true));

fn parameters_from_schema(
  schema: Option<ReferenceOr<Schema>>,
  required: Option<bool>,
  default_description: &Option<String>,
  style: &Option<ParameterStyle>,
  explode: Option<bool>,
) -> Vec<Parameter> {
  let mut parameters = vec![];
  if let Some(schema) = schema {
    match schema {
      ReferenceOr::Reference { _ref } => {
        // don't know what to do with it
      }
      ReferenceOr::Object(schema) => {
        if has_object_validation(&schema) {
          parameters.append(&mut parameter_for_obj(
            &schema,
            required,
            default_description,
            style,
            explode,
          ));
        }
        for sch in subschemas(&schema, "allOf") {
          parameters.append(&mut parameters_from_schema(
            Some(ReferenceOr::Object(sch)),
            required,
            default_description,
            style,
            explode,
          ));
        }
        // optional flattened fields are generated as `anyOf: [<schema>, {}]`
        for sch in subschemas(&schema, "anyOf") {
          parameters.append(&mut parameters_from_schema(
            Some(ReferenceOr::Object(sch)),
            Some(false),
            default_description,
            style,
            explode,
          ));
        }
        let one_of = subschemas(&schema, "oneOf");
        if !one_of.is_empty() {
          let properties = one_of
            .iter()
            .flat_map(|one_of_sch| properties(one_of_sch).into_iter().map(|(name, _)| name))
            .collect::<Vec<_>>();
          let description = format!("{} are mutually exclusive properties", properties.join(", "));
          for one_of_sch in one_of {
            parameters.append(&mut parameters_from_schema(
              Some(ReferenceOr::Object(one_of_sch)),
              Some(false),
              &Some(description.clone()),
              style,
              explode,
            ));
          }
        }
      }
    }
  }
  parameters
}

fn parameters_from_hashmap(schema: Option<ReferenceOr<Schema>>, style: Option<ParameterStyle>) -> Vec<Parameter> {
  let parameters;
  if let Some(schema) = schema {
    match schema {
      ReferenceOr::Reference { .. } => {
        parameters = vec![Parameter {
          name: "params".to_string(),
          _in: ParameterIn::Query,
          definition: Some(ParameterDefinition::Schema(Box::new(ReferenceOr::Object(
            json_schema!({}),
          )))),
          ..Default::default()
        }];
      }
      ReferenceOr::Object(schema) => {
        parameters = vec![Parameter {
          name: "params".to_string(),
          _in: ParameterIn::Query,
          style,
          definition: Some(ParameterDefinition::Schema(Box::new(ReferenceOr::Object(
            json_schema!({ "additionalProperties": schema }),
          )))),
          ..Default::default()
        }];
      }
    }
  } else {
    parameters = vec![Parameter {
      name: "params".to_string(),
      _in: ParameterIn::Query,
      style,
      definition: Some(ParameterDefinition::Schema(Box::new(ReferenceOr::Object(
        json_schema!({}),
      )))),
      ..Default::default()
    }];
  }
  parameters
}

fn parameter_for_obj(
  sch: &Schema,
  required: Option<bool>,
  default_description: &Option<String>,
  style: &Option<ParameterStyle>,
  explode: Option<bool>,
) -> Vec<Parameter> {
  properties(sch)
    .into_iter()
    .map(|(name, schema)| {
      let required = required.or_else(|| extract_required_from_schema(sch, &name));
      let description = schema
        .get("description")
        .and_then(|d| d.as_str())
        .map(ToString::to_string)
        .or_else(|| default_description.clone());
      Parameter {
        name,
        _in: ParameterIn::Query,
        definition: Some(ParameterDefinition::Schema(Box::new(schema.into()))),
        required,
        description,
        style: style.clone(),
        explode,
        ..Default::default()
      }
    })
    .collect()
}

/// Keywords for which the requirement of a property can't be inferred from the parent schema.
const NON_OBJECT_KEYWORDS: [&str; 21] = [
  // subschemas
  "allOf",
  "anyOf",
  "oneOf",
  "not",
  "if",
  "then",
  "else",
  // string
  "maxLength",
  "minLength",
  "pattern",
  // number
  "multipleOf",
  "maximum",
  "exclusiveMaximum",
  "minimum",
  "exclusiveMinimum",
  // array
  "items",
  "additionalItems",
  "maxItems",
  "minItems",
  "uniqueItems",
  // reference
  "$ref",
];

fn extract_required_from_schema(sch: &Schema, property_name: &str) -> Option<bool> {
  let is_required = sch
    .get("required")
    .and_then(|r| r.as_array())
    .is_some_and(|r| r.iter().any(|ri| ri.as_str() == Some(property_name)));
  if is_required {
    return Some(true);
  }
  if NON_OBJECT_KEYWORDS.iter().any(|k| sch.get(*k).is_some()) {
    return None;
  }
  Some(false)
}

#[cfg(test)]
mod test {
  use crate::ApiComponent;
  use actix_web::web::Query;
  #[cfg(feature = "lab_query")]
  use actix_web_lab::extract::Query as LabQuery;
  #[cfg(feature = "lab_query")]
  use apistos_models::paths::ParameterStyle;
  use apistos_models::paths::{Parameter, ParameterDefinition, ParameterIn};
  use apistos_models::reference_or::ReferenceOr;
  use schemars::{JsonSchema, Schema, json_schema};
  use serde::{Deserialize, Serialize};
  #[cfg(feature = "qs_query")]
  use serde_qs::actix::QsQuery;

  #[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
  struct Test {
    id_number: u32,
    id_string: String,
  }

  impl ApiComponent for Test {
    fn child_schemas() -> Vec<(String, ReferenceOr<Schema>)> {
      vec![]
    }

    fn schema() -> Option<(String, ReferenceOr<Schema>)> {
      Some((
        <Self as JsonSchema>::schema_name().to_string(),
        ReferenceOr::Object(apistos_models::schema_for::<Self>()),
      ))
    }
  }

  #[test]
  fn test_query_parameter() {
    let parameters_schema = <Query<Test> as ApiComponent>::parameters();
    assert_eq!(parameters_schema.len(), 2);

    let id_number_parameter_schema = parameters_schema
      .iter()
      .find(|ps| ps.name == *"id_number")
      .unwrap()
      .clone();
    assert_eq!(
      id_number_parameter_schema,
      Parameter {
        name: "id_number".to_string(),
        _in: ParameterIn::Query,
        required: Some(true),
        definition: Some(ParameterDefinition::Schema(Box::new(ReferenceOr::Object(
          json_schema!({
            "type": "integer",
            "format": "uint32",
            "minimum": 0
          })
        )))),
        ..Default::default()
      }
    );

    let id_string_parameter_schema = parameters_schema
      .iter()
      .find(|ps| ps.name == *"id_string")
      .unwrap()
      .clone();
    assert_eq!(
      id_string_parameter_schema,
      Parameter {
        name: "id_string".to_string(),
        _in: ParameterIn::Query,
        required: Some(true),
        definition: Some(ParameterDefinition::Schema(Box::new(ReferenceOr::Object(
          json_schema!({ "type": "string" })
        )))),
        ..Default::default()
      }
    );
  }

  #[cfg(feature = "qs_query")]
  #[test]
  fn test_qs_query_parameter() {
    let parameters_schema = <QsQuery<Test> as ApiComponent>::parameters();
    assert_eq!(parameters_schema.len(), 2);

    let id_number_parameter_schema = parameters_schema
      .iter()
      .find(|ps| ps.name == *"id_number")
      .unwrap()
      .clone();
    assert_eq!(
      id_number_parameter_schema,
      Parameter {
        name: "id_number".to_string(),
        _in: ParameterIn::Query,
        required: Some(true),
        definition: Some(ParameterDefinition::Schema(Box::new(ReferenceOr::Object(
          json_schema!({
            "type": "integer",
            "format": "uint32",
            "minimum": 0
          })
        )))),
        ..Default::default()
      }
    );

    let id_string_parameter_schema = parameters_schema
      .iter()
      .find(|ps| ps.name == *"id_string")
      .unwrap()
      .clone();
    assert_eq!(
      id_string_parameter_schema,
      Parameter {
        name: "id_string".to_string(),
        _in: ParameterIn::Query,
        required: Some(true),
        definition: Some(ParameterDefinition::Schema(Box::new(ReferenceOr::Object(
          json_schema!({ "type": "string" })
        )))),
        ..Default::default()
      }
    );
  }

  #[cfg(feature = "lab_query")]
  #[test]
  fn test_lab_query_parameter() {
    let parameters_schema = <LabQuery<Test> as ApiComponent>::parameters();
    assert_eq!(parameters_schema.len(), 2);

    let id_number_parameter_schema = parameters_schema
      .iter()
      .find(|ps| ps.name == *"id_number")
      .unwrap()
      .clone();
    assert_eq!(
      id_number_parameter_schema,
      Parameter {
        name: "id_number".to_string(),
        _in: ParameterIn::Query,
        required: Some(true),
        style: Some(ParameterStyle::Form),
        explode: Some(true),
        definition: Some(ParameterDefinition::Schema(Box::new(ReferenceOr::Object(
          json_schema!({
            "type": "integer",
            "format": "uint32",
            "minimum": 0
          })
        )))),
        ..Default::default()
      }
    );

    let id_string_parameter_schema = parameters_schema
      .iter()
      .find(|ps| ps.name == *"id_string")
      .unwrap()
      .clone();
    assert_eq!(
      id_string_parameter_schema,
      Parameter {
        name: "id_string".to_string(),
        _in: ParameterIn::Query,
        required: Some(true),
        style: Some(ParameterStyle::Form),
        explode: Some(true),
        definition: Some(ParameterDefinition::Schema(Box::new(ReferenceOr::Object(
          json_schema!({ "type": "string" })
        )))),
        ..Default::default()
      }
    );
  }
}
