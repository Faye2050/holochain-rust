//! dna is a library for working with holochain dna files/entries.
//!
//! It includes utilities for representing dna structures in memory,
//! as well as serializing and deserializing dna, mainly to json format.
//!
//! # Examples
//!
//! ```
//! #![feature(try_from)]
//! extern crate holochain_core_types;
//! use holochain_core_types::dna::Dna;
//! use holochain_core_types::json::JsonString;
//! use std::convert::TryFrom;
//!
//! let name = String::from("My Holochain DNA");
//!
//! let mut dna = Dna::new();
//! dna.name = name.clone();
//!
//! let json = JsonString::from(dna.clone());
//!
//! let dna2 = Dna::try_from(json).expect("could not restore DNA from JSON");
//! assert_eq!(name, dna2.name);
//! ```

pub mod wasm;
pub mod zome;

use crate::{
    dna::zome::{capabilities::Capability, entry_types::EntryTypeDef},
    entry::entry_type::EntryType,
    error::{DnaError, HolochainError},
    json::JsonString,
};
use entry::entry_type::AppEntryType;
use multihash;
use serde_json::{self, Value};
use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
};
use uuid::Uuid;

/// serde helper, provides a default empty object
fn empty_object() -> Value {
    json!({})
}

/// serde helper, provides a default newly generated v4 uuid
fn new_uuid() -> String {
    Uuid::new_v4().to_string()
}

/// Represents the top-level holochain dna object.
#[derive(Serialize, Deserialize, Clone, Debug, DefaultJson)]
pub struct Dna {
    /// The top-level "name" of a holochain application.
    #[serde(default)]
    pub name: String,

    /// The top-level "description" of a holochain application.
    #[serde(default)]
    pub description: String,

    /// The semantic version of your holochain application.
    #[serde(default)]
    pub version: String,

    /// A unique identifier to distinguish your holochain application.
    #[serde(default = "new_uuid")]
    pub uuid: String,

    /// Which version of the holochain dna spec does this represent?
    #[serde(default)]
    pub dna_spec_version: String,

    /// Any arbitrary application properties can be included in this object.
    #[serde(default = "empty_object")]
    pub properties: Value,

    /// An array of zomes associated with your holochain application.
    #[serde(default)]
    pub zomes: BTreeMap<String, zome::Zome>,
}

impl Default for Dna {
    /// Provide defaults for a dna object.
    fn default() -> Self {
        Dna {
            name: String::new(),
            description: String::new(),
            version: String::new(),
            uuid: new_uuid(),
            dna_spec_version: String::from("2.0"),
            properties: empty_object(),
            zomes: BTreeMap::new(),
        }
    }
}

impl Dna {
    /// Create a new in-memory dna structure with some default values.
    ///
    /// # Examples
    ///
    /// ```
    /// use holochain_core_types::dna::Dna;
    ///
    /// let dna = Dna::new();
    /// assert_eq!("", dna.name);
    ///
    /// ```
    pub fn new() -> Self {
        Default::default()
    }

    /// Generate a pretty-printed json string from an in-memory dna struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use holochain_core_types::dna::Dna;
    ///
    /// let dna = Dna::new();
    /// println!("json: {}", dna.to_json_pretty().expect("DNA should serialize"));
    ///
    /// ```
    pub fn to_json_pretty(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// Return a Zome
    pub fn get_zome(&self, zome_name: &str) -> Option<&zome::Zome> {
        self.zomes.get(zome_name)
    }

    /// Return a Zome's Capability from a Zome and a Capability name.
    pub fn get_capability<'a>(
        &'a self,
        zome: &'a zome::Zome,
        capability_name: &str,
    ) -> Option<&'a Capability> {
        zome.capabilities.get(capability_name)
    }

    /// Find a Zome and return it's WASM bytecode for a specified Capability
    pub fn get_wasm_from_zome_name<T: Into<String>>(&self, zome_name: T) -> Option<&wasm::DnaWasm> {
        let zome_name = zome_name.into();
        let zome = self.get_zome(&zome_name)?;
        Some(&zome.code)
    }

    /// Return a Zome's Capability from a Zome name and Capability name.
    pub fn get_capability_with_zome_name(
        &self,
        zome_name: &str,
        cap_name: &str,
    ) -> Result<&Capability, DnaError> {
        // Zome must exist in DNA
        let zome = self.get_zome(zome_name);
        if zome.is_none() {
            return Err(DnaError::ZomeNotFound(format!(
                "Zome '{}' not found",
                &zome_name,
            )));
        }
        let zome = zome.unwrap();
        // Capability must exist in Zome
        let cap = self.get_capability(zome, &cap_name);
        if cap.is_none() {
            return Err(DnaError::CapabilityNotFound(format!(
                "Capability '{}' not found in Zome '{}'",
                &cap_name, &zome_name
            )));
        }
        // Everything OK
        Ok(cap.unwrap())
    }

    /// Return the name of the zome holding a specified app entry_type
    pub fn get_zome_name_for_app_entry_type(
        &self,
        app_entry_type: &AppEntryType,
    ) -> Option<String> {
        let entry_type_name = String::from(app_entry_type.to_owned());
        // pre-condition: must be a valid app entry_type name
        assert!(EntryType::has_valid_app_name(&entry_type_name));
        // Browse through the zomes
        for (zome_name, zome) in &self.zomes {
            for (zome_entry_type_name, _) in &zome.entry_types {
                if *zome_entry_type_name
                    == EntryType::App(AppEntryType::from(entry_type_name.to_string()))
                {
                    return Some(zome_name.clone());
                }
            }
        }
        None
    }

    /// Return the entry_type definition of a specified app entry_type
    pub fn get_entry_type_def(&self, entry_type_name: &str) -> Option<&EntryTypeDef> {
        // pre-condition: must be a valid app entry_type name
        assert!(EntryType::has_valid_app_name(entry_type_name));
        // Browse through the zomes
        for (_zome_name, zome) in &self.zomes {
            for (zome_entry_type_name, entry_type_def) in &zome.entry_types {
                if *zome_entry_type_name
                    == EntryType::App(AppEntryType::from(entry_type_name.to_string()))
                {
                    return Some(entry_type_def);
                }
            }
        }
        None
    }

    pub fn multihash(&self) -> Result<Vec<u8>, HolochainError> {
        let s = String::from(JsonString::from(self.to_owned()));
        multihash::encode(multihash::Hash::SHA2256, &s.into_bytes())
            .map_err(|error| HolochainError::ErrorGeneric(error.to_string()))
    }
}

impl Hash for Dna {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let s = String::from(JsonString::from(self.to_owned()));
        s.hash(state);
    }
}

impl PartialEq for Dna {
    fn eq(&self, other: &Dna) -> bool {
        // need to guarantee that PartialEq and Hash always agree
        JsonString::from(self.to_owned()) == JsonString::from(other.to_owned())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    extern crate base64;
    use crate::dna::zome::tests::test_zome;
    use std::convert::TryFrom;

    static UNIT_UUID: &'static str = "00000000-0000-0000-0000-000000000000";

    pub fn test_dna() -> Dna {
        Dna::new()
    }

    #[test]
    fn get_entry_type_def_test() {
        let mut dna = test_dna();
        let mut zome = test_zome();
        let entry_type = EntryType::App(AppEntryType::from("bar"));
        let entry_type_def = EntryTypeDef::new();

        zome.entry_types
            .insert(entry_type.into(), entry_type_def.clone());
        dna.zomes.insert("zome".to_string(), zome);

        assert_eq!(None, dna.get_entry_type_def("foo"));
        assert_eq!(Some(&entry_type_def), dna.get_entry_type_def("bar"));
    }

    #[test]
    fn can_parse_and_output_json() {
        let dna = test_dna();

        let serialized = serde_json::to_string(&dna).unwrap();

        let deserialized: Dna = serde_json::from_str(&serialized).unwrap();

        assert_eq!(String::from("2.0"), deserialized.dna_spec_version);
    }

    #[test]
    fn can_parse_and_output_json_helpers() {
        let dna = test_dna();

        let json_string = JsonString::from(dna);

        let deserialized = Dna::try_from(json_string).unwrap();

        assert_eq!(String::from("2.0"), deserialized.dna_spec_version);
    }

    #[test]
    fn parse_and_serialize_compare() {
        let fixture = String::from(
            r#"{
                "name": "test",
                "description": "test",
                "version": "test",
                "uuid": "00000000-0000-0000-0000-000000000000",
                "dna_spec_version": "2.0",
                "properties": {
                    "test": "test"
                },
                "zomes": {
                    "test": {
                        "description": "test",
                        "config": {
                            "error_handling": "throw-errors"
                        },
                        "entry_types": {
                            "test": {
                                "description": "test",
                                "sharing": "public",
                                "links_to": [
                                    {
                                        "target_type": "test",
                                        "tag": "test"
                                    }
                                ],
                                "linked_from": []
                            }
                        },
                        "capabilities": {
                            "test": {
                                "capability": {
                                    "membrane": "public"
                                },
                                "functions": [
                                    {
                                        "name": "test",
                                        "inputs": [],
                                        "outputs": []
                                    }
                                ]
                            }
                        },
                        "code": {
                            "code": "AAECAw=="
                        }
                    }
                }
            }"#,
        )
        .replace(char::is_whitespace, "");

        let dna = Dna::try_from(JsonString::from(fixture.clone())).unwrap();

        println!("{}", dna.to_json_pretty().unwrap());

        let serialized = String::from(JsonString::from(dna)).replace(char::is_whitespace, "");

        assert_eq!(fixture, serialized);
    }

    #[test]
    fn default_value_test() {
        let mut dna = Dna {
            uuid: String::from(UNIT_UUID),
            ..Default::default()
        };
        let mut zome = zome::Zome::default();
        zome.entry_types
            .insert("".into(), zome::entry_types::EntryTypeDef::new());
        dna.zomes.insert("".to_string(), zome);

        let expected = JsonString::from(dna.clone());
        println!("{:?}", expected);

        let fixture = Dna::try_from(JsonString::from(
            r#"{
                "name": "",
                "description": "",
                "version": "",
                "uuid": "00000000-0000-0000-0000-000000000000",
                "dna_spec_version": "2.0",
                "properties": {},
                "zomes": {
                    "": {
                        "description": "",
                        "config": {
                            "error_handling": "throw-errors"
                        },
                        "entry_types": {
                            "": {
                                "description": "",
                                "sharing": "public",
                                "links_to": [],
                                "linked_from": []
                            }
                        },
                        "capabilities": {},
                        "code": {"code": ""}
                    }
                }
            }"#,
        ))
        .unwrap();

        assert_eq!(dna, fixture);
    }

    #[test]
    fn parse_with_defaults_dna() {
        let dna = Dna::try_from(JsonString::from(
            r#"{
            }"#,
        ))
        .unwrap();

        assert!(dna.uuid.len() > 0);
    }

    #[test]
    fn parse_with_defaults_zome() {
        let dna = Dna::try_from(JsonString::from(
            r#"{
                "zomes": {
                    "zome1": {}
                }
            }"#,
        ))
        .unwrap();

        assert_eq!(
            dna.zomes.get("zome1").unwrap().config.error_handling,
            zome::ErrorHandling::ThrowErrors
        )
    }

    #[test]
    fn parse_with_defaults_entry_type() {
        let dna = Dna::try_from(JsonString::from(
            r#"{
                "zomes": {
                    "zome1": {
                        "entry_types": {
                            "type1": {}
                        }
                    }
                }
            }"#,
        ))
        .unwrap();

        assert_eq!(
            dna.zomes
                .get("zome1")
                .unwrap()
                .entry_types
                .get(&"type1".into())
                .unwrap()
                .sharing,
            zome::entry_types::Sharing::Public
        );
    }

    #[test]
    fn parse_wasm() {
        let dna = Dna::try_from(JsonString::from(
            r#"{
                "zomes": {
                    "zome1": {
                        "entry_types": {
                            "type1": {}
                        },
                        "code": {
                            "code": "AAECAw=="
                        }
                    }
                }
            }"#,
        ))
        .unwrap();

        assert_eq!(vec![0, 1, 2, 3], dna.zomes.get("zome1").unwrap().code.code);
    }

    #[test]
    #[should_panic]
    fn parse_fail_if_bad_type_dna() {
        Dna::try_from(JsonString::from(
            r#"{
                "name": 42
            }"#,
        ))
        .unwrap();
    }

    #[test]
    #[should_panic]
    fn parse_fail_if_bad_type_zome() {
        Dna::try_from(JsonString::from(
            r#"{
                "zomes": {
                    "zome1": {
                        "description": 42
                    }
                }
            }"#,
        ))
        .unwrap();
    }

    #[test]
    #[should_panic]
    fn parse_fail_if_bad_type_entry_type() {
        Dna::try_from(JsonString::from(
            r#"{
                "zomes": {
                    "zome1": {
                        "entry_types": {
                            "test": {
                                "description": 42
                            }
                        }
                    }
                }
            }"#,
        ))
        .unwrap();
    }

    #[test]
    fn parse_accepts_arbitrary_dna_properties() {
        let dna = Dna::try_from(JsonString::from(
            r#"{
                "properties": {
                    "str": "hello",
                    "num": 3.14159,
                    "bool": true,
                    "null": null,
                    "arr": [1, 2],
                    "obj": {"a": 1, "b": 2}
                }
            }"#,
        ))
        .unwrap();

        let props = dna.properties.as_object().unwrap();

        assert_eq!("hello", props.get("str").unwrap().as_str().unwrap());
        assert_eq!(3.14159, props.get("num").unwrap().as_f64().unwrap());
        assert_eq!(true, props.get("bool").unwrap().as_bool().unwrap());
        assert!(props.get("null").unwrap().is_null());
        assert_eq!(
            1_i64,
            props.get("arr").unwrap().as_array().unwrap()[0]
                .as_i64()
                .unwrap()
        );
        assert_eq!(
            1_i64,
            props
                .get("obj")
                .unwrap()
                .as_object()
                .unwrap()
                .get("a")
                .unwrap()
                .as_i64()
                .unwrap()
        );
    }

    #[test]
    fn get_wasm_from_zome_name() {
        let dna = Dna::try_from(JsonString::from(
            r#"{
                "name": "test",
                "description": "test",
                "version": "test",
                "uuid": "00000000-0000-0000-0000-000000000000",
                "dna_spec_version": "2.0",
                "properties": {
                    "test": "test"
                },
                "zomes": {
                    "test zome": {
                        "name": "test zome",
                        "description": "test",
                        "config": {},
                        "entry_types": {},
                        "capabilities": {
                            "test capability": {
                                "capability": {
                                    "membrane": "public"
                                },
                                "fn_declarations": [
                                    {
                                        "name": "test",
                                        "signature": {
                                            "inputs": [],
                                            "outputs": []
                                        }
                                    }
                                ]
                            }
                        },
                        "code": {
                            "code": "AAECAw=="
                        }
                    }
                }
            }"#,
        ))
        .unwrap();

        let wasm = dna.get_wasm_from_zome_name("test zome");
        assert_eq!("AAECAw==", base64::encode(&wasm.unwrap().code));

        let fail = dna.get_wasm_from_zome_name("non existant zome");
        assert_eq!(None, fail);
    }

    #[test]
    fn test_get_zome_name_for_entry_type() {
        let dna = Dna::try_from(JsonString::from(
            r#"{
                "name": "test",
                "description": "test",
                "version": "test",
                "uuid": "00000000-0000-0000-0000-000000000000",
                "dna_spec_version": "2.0",
                "properties": {
                    "test": "test"
                },
                "zomes": {
                    "test zome": {
                        "name": "test zome",
                        "description": "test",
                        "config": {},
                        "capabilities": {
                            "test capability": {
                                "capability": {
                                    "membrane": "public"
                                },
                                "fn_declarations": []
                            }
                        },
                        "entry_types": {
                            "test type": {
                                "description": "",
                                "sharing": "public"
                            }
                        },
                        "code": {
                            "code": ""
                        }
                    }
                }
            }"#,
        ))
        .unwrap();

        assert_eq!(
            dna.get_zome_name_for_app_entry_type(&AppEntryType::from("test type"))
                .unwrap(),
            "test zome".to_string()
        );
        assert!(
            dna.get_zome_name_for_app_entry_type(&AppEntryType::from("non existant entry type"))
                .is_none()
        );
    }
}
