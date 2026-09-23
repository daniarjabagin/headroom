use headroom_core::units::Tokens;
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::catalog::Catalog;
use crate::error::PricingError;
use crate::money::PicoUsd;
use crate::rates::{ModelRates, RawModel, RawRates};
use crate::source::{PriceSource, Vendor};

const CONTEXT_OVER_200K: Tokens = Tokens(200_000);

#[derive(Debug, Deserialize)]
struct Cost {
    input: f64,
    output: f64,
    cache_read: Option<f64>,
    cache_write: Option<f64>,
    #[serde(default)]
    tiers: Vec<Tier>,
    context_over_200k: Option<TierCost>,
}

#[derive(Debug, Deserialize)]
struct Tier {
    #[serde(flatten)]
    cost: TierCost,
    tier: Option<TierKind>,
}

#[derive(Debug, Deserialize)]
struct TierKind {
    #[serde(rename = "type")]
    kind: Option<String>,
    size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TierCost {
    input: Option<f64>,
    output: Option<f64>,
    cache_read: Option<f64>,
    cache_write: Option<f64>,
}

pub(crate) fn parse(document: &Value) -> Result<Catalog, PricingError> {
    let entries = Vendor::ALL.into_iter().flat_map(|vendor| {
        vendor_models(document, vendor)
            .into_iter()
            .flatten()
            .filter_map(move |(id, model)| {
                parse_model(vendor, id, model).map(|rates| (id.to_lowercase(), rates))
            })
    });
    Catalog::from_entries(PriceSource::ModelsDev, entries)
}

pub(crate) fn trim(document: &Value) -> Value {
    let providers: Map<String, Value> = Vendor::ALL
        .into_iter()
        .map(|vendor| {
            let models: Map<String, Value> = vendor_models(document, vendor)
                .into_iter()
                .flatten()
                .filter_map(|(id, model)| {
                    let cost = model.get("cost")?;
                    let priced = cost.get("input").is_some() && cost.get("output").is_some();
                    priced.then(|| (id.clone(), json!({ "cost": cost })))
                })
                .collect();
            (vendor.id().to_owned(), json!({ "models": models }))
        })
        .collect();
    Value::Object(providers)
}

fn vendor_models(document: &Value, vendor: Vendor) -> Option<&Map<String, Value>> {
    document.get(vendor.id())?.get("models")?.as_object()
}

fn parse_model(vendor: Vendor, id: &str, model: &Value) -> Option<ModelRates> {
    let cost: Cost = serde_json::from_value(model.get("cost")?.clone()).ok()?;
    match raw_model(vendor, &cost) {
        Ok(raw) => raw.complete(),
        Err(error) => {
            tracing::warn!(model = id, %error, "skipping models.dev entry");
            None
        }
    }
}

fn raw_model(vendor: Vendor, cost: &Cost) -> Result<RawModel, PricingError> {
    let mut model = RawModel::new(vendor);
    model.standard = RawRates {
        input: Some(PicoUsd::from_usd_per_million("input", cost.input)?),
        output: Some(PicoUsd::from_usd_per_million("output", cost.output)?),
        cache_read: per_million("cache_read", cost.cache_read)?,
        cache_write_5m: per_million("cache_write", cost.cache_write)?,
        cache_write_1h: None,
    };
    model.long_context = long_context(cost)
        .map(|(threshold, tier)| tier_rates(tier).map(|rates| (threshold, rates)))
        .transpose()?;
    Ok(model)
}

fn long_context(cost: &Cost) -> Option<(Tokens, &TierCost)> {
    let tiered = cost
        .tiers
        .iter()
        .filter_map(|tier| context_size(tier).map(|size| (size, &tier.cost)))
        .min_by_key(|(size, _)| *size);
    tiered.or_else(|| {
        cost.context_over_200k
            .as_ref()
            .map(|tier| (CONTEXT_OVER_200K, tier))
    })
}

fn context_size(tier: &Tier) -> Option<Tokens> {
    let kind = tier.tier.as_ref()?;
    (kind.kind.as_deref() == Some("context"))
        .then_some(kind.size)
        .flatten()
        .map(Tokens)
}

fn tier_rates(tier: &TierCost) -> Result<RawRates, PricingError> {
    Ok(RawRates {
        input: per_million("input", tier.input)?,
        output: per_million("output", tier.output)?,
        cache_read: per_million("cache_read", tier.cache_read)?,
        cache_write_5m: per_million("cache_write", tier.cache_write)?,
        cache_write_1h: None,
    })
}

fn per_million(field: &str, value: Option<f64>) -> Result<Option<PicoUsd>, PricingError> {
    value
        .map(|usd| PicoUsd::from_usd_per_million(field, usd))
        .transpose()
}

#[cfg(test)]
#[path = "models_dev_tests.rs"]
mod tests;
