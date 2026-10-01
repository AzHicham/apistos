use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

pub(crate) struct Schemas {
  pub(crate) deprecated: bool,
}

impl ToTokens for Schemas {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let deprecated = if self.deprecated {
      quote!(
        schema.insert("deprecated".to_owned(), true.into());
      )
    } else {
      quote!()
    };

    tokens.extend(quote! {
      fn child_schemas() -> Vec<(String, apistos::reference_or::ReferenceOr<apistos::Schema>)> {
        let (_, definitions) = apistos::schema_and_definitions_for::<Self>();
        definitions
          .into_iter()
          .map(|(def_name, mut def)| {
            apistos::set_one_of_titles(&mut def);
            (def_name, apistos::reference_or::ReferenceOr::Object(def))
          })
          .collect()
      }

      fn schema() -> Option<(String, apistos::reference_or::ReferenceOr<apistos::Schema>)> {
        let schema_name = <Self as schemars::JsonSchema>::schema_name().to_string();
        let mut schema = apistos::schema_for::<Self>();
        apistos::set_one_of_titles(&mut schema);
        #deprecated
        Some((schema_name, apistos::reference_or::ReferenceOr::Object(schema)))
      }
    });
  }
}
