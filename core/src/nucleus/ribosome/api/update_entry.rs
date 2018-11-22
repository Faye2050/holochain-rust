use nucleus::{
    actions::{build_validation_package::*, validate::*},
    ribosome::{api::ZomeApiResult, Runtime},
};
use agent::actions::{update_entry::update_entry, commit::commit_entry};
use std::convert::TryFrom;
use wasmi::{RuntimeArgs, RuntimeValue};
use futures::{executor::block_on, FutureExt};
use holochain_core_types::{
    cas::content::Address,
    entry::Entry,
    error::HolochainError,
    hash::HashString,
    validation::{EntryAction, EntryLifecycle, ValidationData},
};
use holochain_wasm_utils::api_serialization::UpdateEntryArgs;

/// ZomeApiFunction::UpdateEntry function code
/// args: [0] encoded MemoryAllocation as u32
/// Expected complex argument: UpdateEntryArgs
/// Returns an HcApiReturnCode as I32
pub fn invoke_update_entry(runtime: &mut Runtime, args: &RuntimeArgs) -> ZomeApiResult {
    // deserialize args
    let args_str = runtime.load_json_string_from_args(&args);
    let entry_args = match UpdateEntryArgs::try_from(args_str.clone()) {
        Ok(entry_input) => entry_input,
        // Exit on error
        Err(_) => {
            println!(
                "invoke_update_entry failed to deserialize SerializedEntry: {:?}",
                args_str
            );
            return ribosome_error_code!(ArgumentDeserializationFailed);
        }
    };

    // Create Chain Entry
    let entry = Entry::from(entry_args.new_entry.clone());

    // Wait for future to be resolved
    let task_result: Result<Address, HolochainError> = block_on(
        // 1. Build the context needed for validation of the entry
        build_validation_package(&entry, &runtime.context)
            .and_then(|validation_package| {
                Ok(ValidationData {
                    package: validation_package,
                    sources: vec![HashString::from("<insert your agent key here>")],
                    lifecycle: EntryLifecycle::Chain,
                    action: EntryAction::Commit,
                })
            })
            // 2. Validate the entry
            .and_then(|validation_data| {
                validate_entry(
                    entry.entry_type().clone(),
                    entry.clone(),
                    validation_data,
                    &runtime.context,
                )
            })
            // 3. Commit the valid entry to chain and DHT
            .and_then(|_| {
                commit_entry(
                    entry.clone(),
                    &runtime.context.action_channel,
                    &runtime.context,
                )
            })
            // 3. Update the entry in DHT metadata
            .and_then(|new_address| {
                update_entry(
                    &runtime.context,
                    &runtime.context.action_channel,
                    entry_args.address.clone(),
                    new_address,
                )
            }),
    );

    runtime.store_result(task_result)
}