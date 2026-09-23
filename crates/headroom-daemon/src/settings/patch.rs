use serde_json::{Map, Value};

use super::Settings;
use crate::error::SettingsError;

const REPLACED_WHOLE: &[&str] = &["headline"];

impl Settings {
    pub fn patched(&self, patch_json: &str) -> Result<Settings, SettingsError> {
        let patch: Value = serde_json::from_str(patch_json)?;
        if !patch.is_object() {
            return Err(SettingsError::PatchNotObject);
        }
        let mut document = serde_json::to_value(self)?;
        clear_replaced_whole(&mut document, &patch);
        merge_patch(&mut document, &patch);
        let settings: Settings = serde_json::from_value(document)?;
        settings.validated()
    }
}

fn clear_replaced_whole(document: &mut Value, patch: &Value) {
    let (Value::Object(target), Value::Object(changes)) = (document, patch) else {
        return;
    };
    for key in REPLACED_WHOLE {
        if changes.get(*key).is_some_and(Value::is_object) {
            target.remove(*key);
        }
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
