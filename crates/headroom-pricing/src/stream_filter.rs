use std::fmt;

use serde::Deserializer;
use serde::de::{DeserializeSeed, Error, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

pub(crate) struct ObjectFilter {
    pub(crate) wants: fn(&str) -> bool,
    pub(crate) keep: fn(&str, Value) -> Option<Value>,
}

impl ObjectFilter {
    pub(crate) fn apply(self, body: &[u8]) -> Result<Map<String, Value>, serde_json::Error> {
        let mut deserializer = serde_json::Deserializer::from_slice(body);
        let kept = self.deserialize(&mut deserializer)?;
        deserializer.end()?;
        Ok(kept)
    }
}

impl<'de> DeserializeSeed<'de> for ObjectFilter {
    type Value = Map<String, Value>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for ObjectFilter {
    type Value = Map<String, Value>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON document")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut kept = Map::new();
        while let Some(key) = access.next_key::<String>()? {
            if !(self.wants)(&key) {
                access.next_value::<IgnoredAny>()?;
                kept.remove(&key);
                continue;
            }
            let value = access.next_value::<Value>()?;
            match (self.keep)(&key, value) {
                Some(value) => kept.insert(key, value),
                None => kept.remove(&key),
            };
        }
        Ok(kept)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        while access.next_element::<IgnoredAny>()?.is_some() {}
        Ok(Map::new())
    }

    fn visit_bool<E: Error>(self, _: bool) -> Result<Self::Value, E> {
        Ok(Map::new())
    }

    fn visit_i64<E: Error>(self, _: i64) -> Result<Self::Value, E> {
        Ok(Map::new())
    }

    fn visit_u64<E: Error>(self, _: u64) -> Result<Self::Value, E> {
        Ok(Map::new())
    }

    fn visit_f64<E: Error>(self, _: f64) -> Result<Self::Value, E> {
        Ok(Map::new())
    }

    fn visit_str<E: Error>(self, _: &str) -> Result<Self::Value, E> {
        Ok(Map::new())
    }

    fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
        Ok(Map::new())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn short_strings(_: &str, value: Value) -> Option<Value> {
        match value {
            Value::String(text) if text.len() < 4 => Some(Value::String(text)),
            _ => None,
        }
    }

    fn filter() -> ObjectFilter {
        ObjectFilter {
            wants: |key| !key.starts_with('x'),
            keep: short_strings,
        }
    }

    #[test]
    fn keeps_wanted_entries_accepted_by_keep() {
        let body = br#"{"a":"ok","b":"too long","xa":"ok","c":{"deep":[1,2]}}"#;
        let kept = filter().apply(body).unwrap();
        assert_eq!(Value::Object(kept), json!({ "a": "ok" }));
    }

    #[test]
    fn duplicate_keys_follow_the_last_value() {
        let kept = filter().apply(br#"{"a":"ok","a":"rejected","b":"x","b":"y"}"#);
        assert_eq!(Value::Object(kept.unwrap()), json!({ "b": "y" }));
    }

    #[test]
    fn non_object_documents_yield_nothing() {
        for body in [
            &b"[1,{\"a\":2}]"[..],
            b"null",
            b"true",
            b"-1",
            b"2",
            b"1.5",
            b"\"s\"",
        ] {
            assert!(filter().apply(body).unwrap().is_empty());
        }
    }

    #[test]
    fn malformed_or_trailing_input_is_an_error() {
        assert!(filter().apply(br#"{"a": "#).is_err());
        assert!(filter().apply(br#"{"a":"ok"} {}"#).is_err());
    }
}
