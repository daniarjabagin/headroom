use serde_json::{Map, Value};

const LEGACY_SHOW_USAGE: &str = "show_usage";
const DAEMON_MANAGED: &[&str] = &["dismissed_accounts"];

pub fn upgrade(json: &str) -> Result<Value, serde_json::Error> {
    let mut value: Value = serde_json::from_str(json)?;
    if let Value::Object(settings) = &mut value {
        move_show_usage(settings);
    }
    drop_daemon_managed(&mut value);
    Ok(value)
}

pub fn drop_daemon_managed(settings: &mut Value) {
    if let Value::Object(settings) = settings {
        for key in DAEMON_MANAGED {
            settings.remove(*key);
        }
    }
}

fn move_show_usage(settings: &mut Map<String, Value>) {
    let Some(show_usage) = settings.remove(LEGACY_SHOW_USAGE) else {
        return;
    };
    let display = settings
        .entry("display")
        .or_insert_with(|| Value::Object(Map::new()));
    if let Value::Object(display) = display {
        display.entry("show_spend").or_insert(show_usage);
    }
}
