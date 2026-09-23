use std::collections::BTreeMap;

use headroom_core::event::ServiceTier;
use headroom_core::units::Tokens;
use regex::Regex;
use serde::Deserialize;

use crate::catalog::Catalog;
use crate::error::{PricingError, json_error};
use crate::money::{Multiplier, PicoUsd};
use crate::rates::{ModelRates, RawModel, RawRates};
use crate::source::{PriceSource, Vendor};

#[derive(Debug, Deserialize)]
struct RawSupplement {
    #[serde(default)]
    vendors: BTreeMap<Vendor, RawVendor>,
    #[serde(default)]
    pricing: BTreeMap<String, RawPricing>,
    #[serde(default)]
    tier_multipliers: BTreeMap<String, RawTiers>,
    #[serde(default)]
    strip_suffixes: Vec<String>,
    #[serde(default)]
    aliases: Vec<RawAlias>,
}

#[derive(Debug, Deserialize)]
struct RawVendor {
    web_search_per_request: Option<f64>,
    #[serde(default)]
    tiers: RawTiers,
}

#[derive(Debug, Default, Deserialize)]
struct RawTiers {
    priority: Option<f64>,
    fast: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RawPricing {
    vendor: Vendor,
    #[serde(flatten)]
    rates: RawPerMillion,
    web_search_per_request: Option<f64>,
    long_context: Option<RawLongContext>,
}

#[derive(Debug, Deserialize)]
struct RawLongContext {
    threshold_tokens: u64,
    #[serde(flatten)]
    rates: RawPerMillion,
}

#[derive(Debug, Deserialize)]
struct RawPerMillion {
    #[serde(rename = "input_per_million")]
    input: Option<f64>,
    #[serde(rename = "output_per_million")]
    output: Option<f64>,
    #[serde(rename = "cache_read_per_million")]
    cache_read: Option<f64>,
    #[serde(rename = "cache_write_5m_per_million")]
    cache_write_5m: Option<f64>,
    #[serde(rename = "cache_write_1h_per_million")]
    cache_write_1h: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RawAlias {
    pattern: String,
    canonical: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Tiers {
    priority: Option<Multiplier>,
    fast: Option<Multiplier>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct VendorDefaults {
    web_search: Option<PicoUsd>,
    tiers: Tiers,
}

#[derive(Debug)]
struct Alias {
    pattern: Regex,
    canonical: String,
}

#[derive(Debug)]
pub(crate) struct Supplement {
    vendors: BTreeMap<Vendor, VendorDefaults>,
    pricing: Catalog,
    tier_multipliers: BTreeMap<String, Tiers>,
    strip_suffixes: Vec<String>,
    aliases: Vec<Alias>,
}

impl Supplement {
    pub(crate) fn parse(text: &str) -> Result<Supplement, PricingError> {
        let raw: RawSupplement =
            serde_json::from_str(text).map_err(json_error(PriceSource::Supplement))?;
        Ok(Supplement {
            vendors: raw
                .vendors
                .into_iter()
                .map(|(vendor, defaults)| Ok((vendor, vendor_defaults(&defaults)?)))
                .collect::<Result<_, PricingError>>()?,
            pricing: pricing_catalog(raw.pricing)?,
            tier_multipliers: raw
                .tier_multipliers
                .into_iter()
                .map(|(model, tiers)| Ok((model.to_lowercase(), parse_tiers(&tiers)?)))
                .collect::<Result<_, PricingError>>()?,
            strip_suffixes: raw.strip_suffixes,
            aliases: raw
                .aliases
                .iter()
                .map(compile_alias)
                .collect::<Result<_, _>>()?,
        })
    }

    pub(crate) fn pricing(&self) -> &Catalog {
        &self.pricing
    }

    pub(crate) fn strip_suffixes(&self) -> &[String] {
        &self.strip_suffixes
    }

    pub(crate) fn alias(&self, name: &str) -> Option<&str> {
        self.aliases
            .iter()
            .find(|alias| alias.pattern.is_match(name))
            .map(|alias| alias.canonical.as_str())
    }

    pub(crate) fn model_tier(&self, model: &str, tier: ServiceTier) -> Option<Multiplier> {
        self.tier_multipliers
            .get(model)
            .and_then(|tiers| tiers.get(tier))
    }

    pub(crate) fn vendor_tier(&self, vendor: Vendor, tier: ServiceTier) -> Option<Multiplier> {
        self.vendors
            .get(&vendor)
            .and_then(|defaults| defaults.tiers.get(tier))
    }

    pub(crate) fn vendor_web_search(&self, vendor: Vendor) -> Option<PicoUsd> {
        self.vendors
            .get(&vendor)
            .and_then(|defaults| defaults.web_search)
    }
}

impl Tiers {
    fn get(self, tier: ServiceTier) -> Option<Multiplier> {
        match tier {
            ServiceTier::Standard => Some(Multiplier::ONE),
            ServiceTier::Priority => self.priority,
            ServiceTier::Fast => self.fast,
        }
    }
}

fn vendor_defaults(raw: &RawVendor) -> Result<VendorDefaults, PricingError> {
    Ok(VendorDefaults {
        web_search: raw
            .web_search_per_request
            .map(|usd| PicoUsd::from_usd("web_search_per_request", usd))
            .transpose()?,
        tiers: parse_tiers(&raw.tiers)?,
    })
}

fn parse_tiers(raw: &RawTiers) -> Result<Tiers, PricingError> {
    let parse = |field: &str, value: Option<f64>| {
        value
            .map(|factor| Multiplier::parse(field, factor))
            .transpose()
    };
    Ok(Tiers {
        priority: parse("priority", raw.priority)?,
        fast: parse("fast", raw.fast)?,
    })
}

fn compile_alias(raw: &RawAlias) -> Result<Alias, PricingError> {
    let pattern = Regex::new(&raw.pattern).map_err(|source| PricingError::AliasPattern {
        pattern: raw.pattern.clone(),
        source,
    })?;
    Ok(Alias {
        pattern,
        canonical: raw.canonical.to_lowercase(),
    })
}

fn pricing_catalog(raw: BTreeMap<String, RawPricing>) -> Result<Catalog, PricingError> {
    let mut entries = Vec::with_capacity(raw.len());
    for (model, pricing) in raw {
        let rates = model_rates(&pricing)?.ok_or_else(|| PricingError::IncompleteRates {
            model: model.clone(),
        })?;
        entries.push((model.to_lowercase(), rates));
    }
    Ok(Catalog::from_supplement(entries))
}

fn model_rates(raw: &RawPricing) -> Result<Option<ModelRates>, PricingError> {
    let mut model = RawModel::new(raw.vendor);
    model.standard = per_million_rates(&raw.rates)?;
    model.web_search = raw
        .web_search_per_request
        .map(|usd| PicoUsd::from_usd("web_search_per_request", usd))
        .transpose()?;
    model.long_context = raw
        .long_context
        .as_ref()
        .map(|long| {
            Ok((
                Tokens(long.threshold_tokens),
                per_million_rates(&long.rates)?,
            ))
        })
        .transpose()?;
    Ok(model.complete())
}

fn per_million_rates(raw: &RawPerMillion) -> Result<RawRates, PricingError> {
    let rate = |field: &str, value: Option<f64>| {
        value
            .map(|usd| PicoUsd::from_usd_per_million(field, usd))
            .transpose()
    };
    Ok(RawRates {
        input: rate("input_per_million", raw.input)?,
        output: rate("output_per_million", raw.output)?,
        cache_read: rate("cache_read_per_million", raw.cache_read)?,
        cache_write_5m: rate("cache_write_5m_per_million", raw.cache_write_5m)?,
        cache_write_1h: rate("cache_write_1h_per_million", raw.cache_write_1h)?,
    })
}

#[cfg(test)]
#[path = "supplement_tests.rs"]
mod tests;
