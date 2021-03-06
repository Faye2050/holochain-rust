extern crate serde_json;
use crate::{
    context::Context,
    nucleus::{
        ribosome::{
            self,
            callback::{links_utils, CallbackResult},
        },
        ZomeFnCall,
    },
};
use holochain_core_types::{
    entry::{entry_type::EntryType, Entry},
    error::HolochainError,
    json::JsonString,
    validation::ValidationPackageDefinition,
};
use holochain_wasm_utils::api_serialization::validation::LinkValidationPackageArgs;
use std::{convert::TryFrom, sync::Arc};

pub fn get_validation_package_definition(
    entry: &Entry,
    context: Arc<Context>,
) -> Result<CallbackResult, HolochainError> {
    let dna = context.get_dna().expect("Callback called without DNA set!");
    let result = match entry.entry_type().clone() {
        EntryType::App(app_entry_type) => {
            let zome_name = dna.get_zome_name_for_app_entry_type(&app_entry_type);
            if zome_name.is_none() {
                return Ok(CallbackResult::NotImplemented);
            }

            let zome_name = zome_name.unwrap();
            let wasm = context
                .get_wasm(&zome_name)
                .ok_or(HolochainError::ErrorGeneric(String::from("no wasm found")))?;

            ribosome::run_dna(
                &dna.name.clone(),
                context,
                wasm.code.clone(),
                &ZomeFnCall::new(
                    &zome_name,
                    "no capability, since this is an entry validation call",
                    "__hdk_get_validation_package_for_entry_type",
                    app_entry_type.to_string(),
                ),
                Some(app_entry_type.to_string().into_bytes()),
            )?
        }
        EntryType::LinkAdd => {
            let link_add = match entry {
                Entry::LinkAdd(link_add) => link_add,
                _ => {
                    return Err(HolochainError::ValidationFailed(
                        "Failed to extract LinkAdd".into(),
                    ));
                }
            };
            let (base, target) = links_utils::get_link_entries(link_add.link(), &context)?;

            let link_definition_path = links_utils::find_link_definition_in_dna(
                &base.entry_type(),
                link_add.link().tag(),
                &target.entry_type(),
                &context,
            )
            .map_err(|_| HolochainError::NotImplemented)?;

            let wasm = context
                .get_wasm(&link_definition_path.zome_name)
                .expect("Couldn't get WASM for zome");

            let params = LinkValidationPackageArgs {
                entry_type: link_definition_path.entry_type_name,
                tag: link_definition_path.tag,
                direction: link_definition_path.direction,
            };

            let call = ZomeFnCall::new(
                "",
                "no capability, since this is an entry validation call",
                "__hdk_get_validation_package_for_link",
                params,
            );

            ribosome::run_dna(
                &dna.name.clone(),
                context,
                wasm.code.clone(),
                &call,
                Some(call.parameters.into_bytes()),
            )?
        }
        EntryType::Deletion => JsonString::from(ValidationPackageDefinition::ChainFull),
        _ => Err(HolochainError::NotImplemented)?,
    };

    if result.is_null() {
        Err(HolochainError::SerializationError(String::from(
            "__hdk_get_validation_package_for_entry_type returned empty result",
        )))
    } else {
        match ValidationPackageDefinition::try_from(result) {
            Ok(package) => Ok(CallbackResult::ValidationPackageDefinition(package)),
            Err(_) => Err(HolochainError::SerializationError(String::from(
                "validation_package result could not be deserialized as ValidationPackage",
            ))),
        }
    }
}
