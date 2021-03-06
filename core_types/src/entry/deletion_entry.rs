use crate::{cas::content::Address, error::HolochainError, json::JsonString};

//-------------------------------------------------------------------------------------------------
// LinkAddEntry
//-------------------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, DefaultJson)]
pub struct DeletionEntry {
    deleted_entry_address: Address,
}

impl DeletionEntry {
    pub fn new(deleted_entry_address: Address) -> Self {
        DeletionEntry {
            deleted_entry_address,
        }
    }

    pub fn deleted_entry_address(self) -> Address {
        self.deleted_entry_address
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::{cas::content::AddressableContent, entry::test_entry_a};

    pub fn test_deletion_entry() -> DeletionEntry {
        let entry = test_entry_a();
        DeletionEntry::new(entry.address())
    }

    #[test]
    fn deletion_entry_smoke_test() {
        assert_eq!(
            test_entry_a().address(),
            test_deletion_entry().deleted_entry_address()
        );
    }
}
