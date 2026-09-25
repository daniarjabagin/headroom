use serde_json::{Map, Value};

use super::{Settings, migrate};
use crate::error::SettingsError;

const REPLACED_WHOLE: &[&[&str]] = &[&["headline"], &["display", "panel_position"]];

impl Settings {
    pub fn patched(&self, patch_json: &str) -> Result<Settings, SettingsError> {
        let mut patch: Value = serde_json::from_str(patch_json)?;
        if !patch.is_object() {
            return Err(SettingsError::PatchNotObject);
        }
        migrate::drop_daemon_managed(&mut patch);
        let mut document = serde_json::to_value(self)?;
        for keys in REPLACED_WHOLE {
            clear_replaced_whole(&mut document, &patch, keys);
        }
        merge_patch(&mut document, &patch);
        let settings: Settings = serde_json::from_value(document)?;
        settings.validated()
    }
}

fn clear_replaced_whole(document: &mut Value, patch: &Value, keys: &[&str]) {
    let (Value::Object(target), Value::Object(changes), Some((key, rest))) =
        (document, patch, keys.split_first())
    else {
        return;
    };
    let Some(change) = changes.get(*key) else {
        return;
    };
    if rest.is_empty() {
        if change.is_object() {
            target.remove(*key);
        }
    } else if let Some(inner) = target.get_mut(*key) {
        clear_replaced_whole(inner, change, rest);
    }
}

fn merge_patch(target: &mut Value, patch: &Value) {
    let Value::Object(changes) = patch else {
        *target = patch.clone();
        return;
    };
    if !target.is_object() {
        *target = Value::Object(Map::new());
    }
    if let Value::Object(fields) = target {
        for (key, change) in changes {
            if change.is_null() {
                fields.remove(key);
            } else {
                merge_patch(fields.entry(key.clone()).or_insert(Value::Null), change);
            }
        }
    }
}

#[cfg(test)]
#[path = "patch_tests.rs"]
mod tests;
