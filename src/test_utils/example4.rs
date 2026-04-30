//! Example config with a map-of-structs field — for table-of-tables serialization tests.

use std::collections::BTreeMap;

use crate as confique;
use crate::Config;

/// A cluster configuration with host entries.
#[derive(Debug, Clone, PartialEq, Config)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Conf {
    /// Cluster name.
    #[config(default = "my-cluster")]
    pub name: String,

    /// Host configurations.
    pub hosts: BTreeMap<String, Host>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Host {
    pub root_img: String,
    #[cfg_attr(feature = "serialize", serde(skip_serializing_if = "Option::is_none"))]
    pub num_drives: Option<u32>,
}
