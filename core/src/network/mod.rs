pub mod actions;
pub mod direct_message;
pub mod entry_with_header;
pub mod handler;
pub mod reducers;
pub mod state;

#[cfg(test)]
pub mod tests {
    use crate::{
        instance::tests::test_instance_and_context_by_name,
        network::actions::{get_entry::get_entry, get_validation_package::get_validation_package},
        workflows::author_entry::author_entry,
    };
    use futures::executor::block_on;
    use holochain_core_types::{
        cas::content::AddressableContent,
        crud_status::{create_crud_status_eav, CrudStatus},
        entry::test_entry,
    };
    use test_utils::*;

    #[test]
    fn get_entry_roundtrip() {
        let mut dna = create_test_dna_with_wat("test_zome", "test_cap", None);
        dna.uuid = String::from("get_entry_roundtrip");
        let (_, context1) = test_instance_and_context_by_name(dna.clone(), "alice1").unwrap();
        let (_, context2) = test_instance_and_context_by_name(dna.clone(), "bob1").unwrap();

        // Create Entry & crud-status metadata, and store it.
        let entry = test_entry();
        let result = context1.file_storage.write().unwrap().add(&entry);
        assert!(result.is_ok());
        let status_eav = create_crud_status_eav(&entry.address(), CrudStatus::LIVE);
        let result = context1.eav_storage.write().unwrap().add_eav(&status_eav);
        assert!(result.is_ok());

        // Get it.
        let result = block_on(get_entry(&context2, &entry.address()));
        assert!(result.is_ok());
        let maybe_entry_with_meta = result.unwrap();
        assert!(maybe_entry_with_meta.is_some());
        let entry_with_meta = maybe_entry_with_meta.unwrap();
        assert_eq!(entry_with_meta.entry, entry);
        assert_eq!(entry_with_meta.crud_status, CrudStatus::LIVE);
    }

    #[test]
    fn get_non_existant_entry() {
        let mut dna = create_test_dna_with_wat("test_zome", "test_cap", None);
        dna.uuid = String::from("get_non_existant_entry");
        let (_, _) = test_instance_and_context_by_name(dna.clone(), "alice2").unwrap();
        let (_, context2) = test_instance_and_context_by_name(dna.clone(), "bob2").unwrap();

        let entry = test_entry();

        let result = block_on(get_entry(&context2, &entry.address()));
        assert!(result.is_ok());
        let maybe_entry_with_meta = result.unwrap();
        assert!(maybe_entry_with_meta.is_none());
    }

    #[test]
    fn get_when_alone() {
        let mut dna = create_test_dna_with_wat("test_zome", "test_cap", None);
        dna.uuid = String::from("get_when_alone");
        let (_, context1) = test_instance_and_context_by_name(dna.clone(), "bob3").unwrap();

        let entry = test_entry();

        let result = block_on(get_entry(&context1, &entry.address()));
        assert!(result.is_ok());
        let maybe_entry_with_meta = result.unwrap();
        assert!(maybe_entry_with_meta.is_none());
    }

    #[test]
    fn get_validation_package_roundtrip() {
        let wat = r#"
(module

    (memory 1)
    (export "memory" (memory 0))

    (func
        (export "__hdk_validate_app_entry")
        (param $allocation i32)
        (result i32)

        (i32.const 0)
    )

    (func
        (export "__hdk_validate_link")
        (param $allocation i32)
        (result i32)

        (i32.const 0)
    )


    (func
        (export "__hdk_get_validation_package_for_entry_type")
        (param $allocation i32)
        (result i32)

        ;; This writes "Entry" into memory
        (i32.store (i32.const 0) (i32.const 34))
        (i32.store (i32.const 1) (i32.const 69))
        (i32.store (i32.const 2) (i32.const 110))
        (i32.store (i32.const 3) (i32.const 116))
        (i32.store (i32.const 4) (i32.const 114))
        (i32.store (i32.const 5) (i32.const 121))
        (i32.store (i32.const 6) (i32.const 34))

        (i32.const 7)
    )

    (func
        (export "__hdk_get_validation_package_for_link")
        (param $allocation i32)
        (result i32)

        ;; This writes "Entry" into memory
        (i32.store (i32.const 0) (i32.const 34))
        (i32.store (i32.const 1) (i32.const 69))
        (i32.store (i32.const 2) (i32.const 110))
        (i32.store (i32.const 3) (i32.const 116))
        (i32.store (i32.const 4) (i32.const 114))
        (i32.store (i32.const 5) (i32.const 121))
        (i32.store (i32.const 6) (i32.const 34))

        (i32.const 7)
    )

    (func
        (export "__list_capabilities")
        (param $allocation i32)
        (result i32)

        (i32.const 0)
    )
)
                "#;

        let mut dna = create_test_dna_with_wat("test_zome", "test_cap", Some(wat));
        dna.uuid = String::from("get_validation_package_roundtrip");
        let (_, context1) = test_instance_and_context_by_name(dna.clone(), "alice1").unwrap();

        let entry = test_entry();
        block_on(author_entry(&entry, None, &context1)).expect("Could not author entry");

        let agent1_state = context1.state().unwrap().agent();
        let header = agent1_state
            .chain()
            .iter_type(&agent1_state.top_chain_header(), &entry.entry_type())
            .find(|h| h.entry_address() == &entry.address())
            .expect("There must be a header in the author's source chain after commit");

        let (_, context2) = test_instance_and_context_by_name(dna.clone(), "bob1").unwrap();
        let result = block_on(get_validation_package(header.clone(), &context2));

        assert!(result.is_ok());
        let maybe_validation_package = result.unwrap();
        assert!(maybe_validation_package.is_some());
        let validation_package = maybe_validation_package.unwrap();
        assert_eq!(validation_package.chain_header, Some(header));
    }
}
