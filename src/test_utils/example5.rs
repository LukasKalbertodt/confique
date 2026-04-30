//! Example config with an array-of-structs field — for array-of-tables serialization tests.

use crate as confique;
use crate::Config;

/// A config with a `Vec<Hook>` lifecycle field.
#[derive(Debug, Clone, PartialEq, Config)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Conf {
    /// Cluster name.
    #[config(default = "my-cluster")]
    pub name: String,

    /// Hooks to run after init.
    pub post_init: Vec<Hook>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Hook {
    pub kind: String,
    #[cfg_attr(feature = "serialize", serde(skip_serializing_if = "Option::is_none"))]
    pub exec: Option<String>,
}
