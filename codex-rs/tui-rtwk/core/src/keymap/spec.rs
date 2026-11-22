//! What the user wrote under `[tui.keymap]`: tables of actions, each a key or
//! a list of keys. The host serializes the loaded config (`TuiKeymap`) to JSON
//! and this reads it back; the core never names the config crate.

use std::collections::BTreeMap;

use serde_json::Value;

/// The `tui.keymap.*` entries a user set. An entry with an empty list is an
/// explicit unbind; an action absent here keeps its built-in default.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeymapSpec {
    /// (table, action) to the specs in the order written.
    entries: BTreeMap<(String, String), Vec<String>>,
}

impl KeymapSpec {
    /// The JSON the config serializes to: `{"list": {"accept": ["enter"]}}`.
    /// `null` is an unset entry; a string is one spec; an array is many.
    pub fn from_json(value: &Value) -> KeymapSpec {
        let mut spec = KeymapSpec::default();
        let Some(tables) = value.as_object() else {
            return spec;
        };
        for (table, actions) in tables {
            let Some(actions) = actions.as_object() else {
                continue;
            };
            for (action, keys) in actions {
                match keys {
                    Value::String(key) => spec.insert(table, action, vec![key.clone()]),
                    Value::Array(items) => {
                        let keys = items
                            .iter()
                            .filter_map(|item| item.as_str().map(str::to_owned))
                            .collect();
                        spec.insert(table, action, keys);
                    }
                    _ => {}
                }
            }
        }
        spec
    }

    /// Sets `tui.keymap.<table>.<action>` (`&[]` unbinds it).
    #[must_use]
    pub fn with(mut self, table: &str, action: &str, keys: &[&str]) -> KeymapSpec {
        self.insert(
            table,
            action,
            keys.iter().map(|key| (*key).to_owned()).collect(),
        );
        self
    }

    fn insert(&mut self, table: &str, action: &str, keys: Vec<String>) {
        self.entries
            .insert((table.to_owned(), action.to_owned()), keys);
    }

    /// What the user set for one entry; `None` when unset.
    pub fn get(&self, table: &str, action: &str) -> Option<&[String]> {
        self.entries
            .get(&(table.to_owned(), action.to_owned()))
            .map(Vec::as_slice)
    }

    /// Every key written anywhere, as text.
    pub(super) fn all_specs(&self) -> impl Iterator<Item = &str> {
        self.entries.values().flatten().map(String::as_str)
    }

    pub(super) fn without(&self, table: &str, actions: &[&str]) -> KeymapSpec {
        let mut spec = self.clone();
        for action in actions {
            spec.entries
                .remove(&(table.to_owned(), (*action).to_owned()));
        }
        spec
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
