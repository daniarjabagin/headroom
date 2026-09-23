use headroom_core::units::Tokens;
use serde_json::{Map, Value};

use crate::catalog::Catalog;
use crate::error::PricingError;
use crate::money::{Multiplier, PicoUsd};
use crate::rates::{RawModel, RawRates};
use crate::source::{PriceSource, Vendor};
use crate::stream_filter::ObjectFilter;

const PROVIDER: &str = "litellm_provider";
const MODE: &str = "mode";
const WEB_SEARCH: &str = "search_context_cost_per_query";
const WEB_SEARCH_SIZE: &str = "search_context_size_medium";
const PROVIDER_SPECIFIC: &str = "provider_specific_entry";
const FAST: &str = "fast";
const FINE_TUNE_PREFIX: &str = "ft:";
const MODES: [&str; 2] = ["chat", "responses"];
const INPUT: &str = "input_cost_per_token";
const OUTPUT: &str = "output_cost_per_token";
const CACHE_READ: &str = "cache_read_input_token_cost";
const CACHE_WRITE_5M: &str = "cache_creation_input_token_cost";
const CACHE_WRITE_1H: &str = "cache_creation_input_token_cost_above_1hr";
const RATE_FIELDS: [&str; 5] = [INPUT, OUTPUT, CACHE_READ, CACHE_WRITE_5M, CACHE_WRITE_1H];

pub(crate) fn parse(document: &Value) -> Result<Catalog, PricingError> {
    let entries = document
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(key, value)| {
            let entry = priced_entry(key, value)?;
            parse_entry(key, entry).map(|rates| (key.to_lowercase(), rates))
        });
    Catalog::from_entries(PriceSource::LiteLlm, entries)
}

pub(crate) fn trim(body: &[u8]) -> Result<Value, serde_json::Error> {
    let filter = ObjectFilter {
        wants: |key| !key.starts_with(FINE_TUNE_PREFIX),
        keep: trim_entry,
    };
    filter.apply(body).map(Value::Object)
}

fn trim_entry(key: &str, value: Value) -> Option<Value> {
    priced_entry(key, &value)?;
    let Value::Object(entry) = value else {
        return None;
    };
    let fields = entry
        .into_iter()
        .filter(|(name, _)| is_kept_field(name))
        .collect();
    Some(Value::Object(fields))
}

fn priced_entry<'a>(key: &str, value: &'a Value) -> Option<&'a Map<String, Value>> {
    let entry = value.as_object()?;
    let vendor_known = entry
        .get(PROVIDER)
        .and_then(Value::as_str)
        .and_then(Vendor::from_id)
        .is_some();
    let mode_known = entry
        .get(MODE)
        .and_then(Value::as_str)
        .is_some_and(|mode| MODES.contains(&mode));
    let usable = vendor_known
        && mode_known
        && entry.contains_key(INPUT)
        && !key.starts_with(FINE_TUNE_PREFIX);
    usable.then_some(entry)
}

fn is_kept_field(name: &str) -> bool {
    [PROVIDER, MODE, WEB_SEARCH, PROVIDER_SPECIFIC].contains(&name)
        || RATE_FIELDS.contains(&name)
        || long_context_field(name).is_some()
}

fn parse_entry(key: &str, entry: &Map<String, Value>) -> Option<crate::rates::ModelRates> {
    let vendor = entry
        .get(PROVIDER)
        .and_then(Value::as_str)
        .and_then(Vendor::from_id)?;
    match raw_model(vendor, entry) {
        Ok(raw) => raw.complete(),
        Err(error) => {
            tracing::warn!(model = key, %error, "skipping LiteLLM entry");
            None
        }
    }
}

fn raw_model(vendor: Vendor, entry: &Map<String, Value>) -> Result<RawModel, PricingError> {
    let mut model = RawModel::new(vendor);
    for (name, value) in entry {
        let Some(number) = value.as_f64() else {
            continue;
        };
        if let Some(slot) = rate_slot(&mut model.standard, name) {
            *slot = Some(PicoUsd::from_usd(name, number)?);
        } else if let Some((base, threshold)) = long_context_field(name)
            && let Some(slot) = long_rates(&mut model, threshold).and_then(|r| rate_slot(r, base))
        {
            *slot = Some(PicoUsd::from_usd(name, number)?);
        }
    }
    model.web_search = web_search_rate(entry)?;
    model.fast_multiplier = fast_multiplier(entry)?;
    Ok(model)
}

fn long_rates(model: &mut RawModel, threshold: Tokens) -> Option<&mut RawRates> {
    match model.long_context {
        Some((existing, _)) if existing < threshold => return None,
        Some((existing, _)) if existing == threshold => {}
        _ => model.long_context = Some((threshold, RawRates::default())),
    }
    model.long_context.as_mut().map(|(_, rates)| rates)
}

fn rate_slot<'a>(rates: &'a mut RawRates, field: &str) -> Option<&'a mut Option<PicoUsd>> {
    match field {
        INPUT => Some(&mut rates.input),
        OUTPUT => Some(&mut rates.output),
        CACHE_READ => Some(&mut rates.cache_read),
        CACHE_WRITE_5M => Some(&mut rates.cache_write_5m),
        CACHE_WRITE_1H => Some(&mut rates.cache_write_1h),
        _ => None,
    }
}

fn long_context_field(name: &str) -> Option<(&str, Tokens)> {
    let (base, thousands) = name.strip_suffix("k_tokens")?.rsplit_once("_above_")?;
    let thousands: u64 = thousands.parse().ok()?;
    RATE_FIELDS
        .contains(&base)
        .then_some((base, Tokens(thousands.checked_mul(1000)?)))
}

fn web_search_rate(entry: &Map<String, Value>) -> Result<Option<PicoUsd>, PricingError> {
    entry
        .get(WEB_SEARCH)
        .and_then(|costs| costs.get(WEB_SEARCH_SIZE))
        .and_then(Value::as_f64)
        .map(|usd| PicoUsd::from_usd(WEB_SEARCH, usd))
        .transpose()
}

fn fast_multiplier(entry: &Map<String, Value>) -> Result<Option<Multiplier>, PricingError> {
    entry
        .get(PROVIDER_SPECIFIC)
        .and_then(|specific| specific.get(FAST))
        .and_then(Value::as_f64)
        .map(|factor| Multiplier::parse(FAST, factor))
        .transpose()
}

#[cfg(test)]
#[path = "litellm_tests.rs"]
mod tests;
