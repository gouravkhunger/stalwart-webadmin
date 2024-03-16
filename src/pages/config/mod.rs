pub mod edit;
pub mod list;
pub mod schema;

use crate::{
    components::{
        icon::{IconCircleStack, IconServerStack, IconShieldCheck, IconUserGroup},
        layout::{LayoutBuilder, MenuItem},
    },
    core::{
        form::{FormData, FormValue},
        schema::*,
    },
};
use ahash::AHashMap;
use leptos::view;
use serde::{Deserialize, Serialize};

pub type Settings = AHashMap<String, String>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UpdateSettings {
    Delete {
        keys: Vec<String>,
    },
    Clear {
        prefix: String,
    },
    Insert {
        prefix: Option<String>,
        values: Vec<(String, String)>,
        assert_empty: bool,
    },
}

impl FormData {
    pub fn build_update(&self) -> Vec<UpdateSettings> {
        let mut updates = Vec::new();
        let mut insert_prefix = None;
        let mut assert_empty = false;

        match &self.schema.typ {
            SchemaType::Record { prefix, .. } => {
                if self.is_update {
                    updates.push(UpdateSettings::Clear {
                        prefix: format!("{prefix}.{}.", self.value_as_str("_id").unwrap()),
                    });
                } else {
                    assert_empty = true;
                }

                insert_prefix = format!("{prefix}.{}", self.value_as_str("_id").unwrap()).into();
            }
            SchemaType::Entry { prefix } => {
                updates.push(UpdateSettings::Insert {
                    prefix: None,
                    assert_empty: !self.is_update,
                    values: vec![(
                        format!("{prefix}.{}", self.value_as_str("_id").unwrap()),
                        self.value_as_str("_value").unwrap_or_default().to_string(),
                    )],
                });
                return updates;
            }
            SchemaType::List => {
                if self.is_update {
                    let mut delete_keys = Vec::new();
                    for field in self.schema.fields.values() {
                        if field.is_multivalue() {
                            updates.push(UpdateSettings::Clear {
                                prefix: format!("{}.", field.id),
                            });
                            delete_keys.push(field.id.to_string());
                        } else if self.value_is_empty(field.id) {
                            delete_keys.push(field.id.to_string());
                        }
                    }

                    if !delete_keys.is_empty() {
                        updates.push(UpdateSettings::Delete { keys: delete_keys });
                    }
                }
            }
        }

        let mut key_values = Vec::new();
        for (key, value) in &self.values {
            if key.starts_with('_') {
                continue;
            }

            match value {
                FormValue::Value(value) if !value.is_empty() => {
                    key_values.push((key.to_string(), value.to_string()));
                }
                FormValue::Array(values) if !values.is_empty() => {
                    let total_values = values.len();
                    if total_values > 1 {
                        let pad_len = (total_values - 1).to_string().len();

                        for (idx, value) in values.iter().enumerate() {
                            key_values.push((format!("{key}.{idx:0>pad_len$}"), value.to_string()));
                        }
                    } else {
                        key_values.push((key.to_string(), values.first().unwrap().to_string()));
                    }
                }
                FormValue::Expression(expr) if !expr.is_empty() => {
                    if !expr.if_thens.is_empty() {
                        let total_values = expr.if_thens.len();
                        let pad_len = total_values.to_string().len();

                        for (idx, if_then) in expr.if_thens.iter().enumerate() {
                            key_values.push((
                                format!("{key}.{idx:0>pad_len$}.if"),
                                if_then.if_.to_string(),
                            ));
                            key_values.push((
                                format!("{key}.{idx:0>pad_len$}.then"),
                                if_then.then_.to_string(),
                            ));
                        }

                        key_values.push((
                            format!("{key}.{total_values:0>pad_len$}.else"),
                            expr.else_.to_string(),
                        ));
                    } else {
                        key_values.push((key.to_string(), expr.else_.to_string()));
                    }
                }
                _ => (),
            }
        }

        if !key_values.is_empty() {
            updates.push(UpdateSettings::Insert {
                prefix: insert_prefix,
                values: key_values,
                assert_empty,
            });
        }

        updates
    }
}

pub trait SettingsValues {
    fn array_values(&self, prefix: &str) -> Vec<(&str, &str)>;
    fn format(&self, field: &Field) -> String;
}

impl SettingsValues for Settings {
    fn array_values(&self, key: &str) -> Vec<(&str, &str)> {
        let full_prefix = key;
        let prefix = format!("{key}.");

        let mut results = self
            .iter()
            .filter_map(move |(key, value)| {
                if key.starts_with(&prefix) || key == full_prefix {
                    (key.as_str(), value.as_str()).into()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // Sort by key
        results.sort_by(|(l_key, _), (r_key, _)| l_key.cmp(r_key));
        results
    }

    fn format(&self, field: &Field) -> String {
        match &field.typ_ {
            Type::Select {
                source: Source::Static(items),
                multi: false,
            } => {
                let value = self
                    .get(field.id)
                    .map(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string();
                items
                    .iter()
                    .find_map(|(k, v)| if k == &value { Some(*v) } else { None })
                    .map(|s| s.to_string())
                    .unwrap_or(value)
            }
            Type::Array => self
                .array_values(field.id)
                .first()
                .map(|(_, v)| v.to_string())
                .unwrap_or_default(),

            _ => self
                .get(field.id)
                .map(|s| s.as_str())
                .unwrap_or_default()
                .to_string(),
        }
    }
}

impl LayoutBuilder {
    pub fn settings() -> Vec<MenuItem> {
        LayoutBuilder::new("/settings")
            // Server
            .create("Server")
            .icon(view! { <IconServerStack/> })
            // Network
            .create("Network")
            .route("/network/edit")
            .insert()
            // Listener
            .create("Listeners")
            .route("/listener")
            .insert()
            // TLS
            .create("TLS")
            .create("Settings")
            .route("/tls/edit")
            .insert()
            .create("ACME Providers")
            .route("/acme")
            .insert()
            .create("Certificates")
            .route("/certificate")
            .insert()
            .insert()
            // System
            .create("System")
            .route("/system/edit")
            .insert()
            .insert()
            // Stores
            .create("Stores")
            .icon(view! { <IconCircleStack/> })
            .route("/store")
            .insert()
            // Directories
            .create("Directories")
            .icon(view! { <IconUserGroup/> })
            .route("/directory")
            .insert()
            // SPAM Filter
            .create("SPAM Filter")
            .icon(view! { <IconShieldCheck/> })
            .create("Scores")
            .route("/spam-scores")
            .insert()
            .create("Free domains")
            .route("/spam-free")
            .insert()
            .insert()
            .menu_items
    }
}
