use std::path::Path;

use headroom_core::event::ServiceTier;
use headroom_core::tokens::TokenCounts;
use headroom_core::units::MicroUsd;
use headroom_core::usage::PriceBook;
use serde::Serialize;

use crate::cache::{self, Feed};
use crate::catalog::Catalog;
use crate::error::PricingError;
use crate::money::{Multiplier, PicoUsd};
use crate::normalize::{normalize, stems, strip_date};
use crate::rates::ModelRates;
use crate::source::PriceSource;
use crate::supplement::Supplement;

const BUNDLED_SUPPLEMENT: &str = include_str!("../resources/supplement.json");

#[derive(Debug)]
pub struct PriceCatalog {
    supplement: Supplement,
    litellm: Catalog,
    models_dev: Catalog,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedModel {
    pub key: String,
    pub source: PriceSource,
    pub rates: ModelRates,
}

impl PriceCatalog {
    pub fn bundled() -> Result<PriceCatalog, PricingError> {
        Ok(PriceCatalog {
            supplement: Supplement::parse(BUNDLED_SUPPLEMENT)?,
            litellm: Feed::LiteLlm.bundled_catalog()?,
            models_dev: Feed::ModelsDev.bundled_catalog()?,
        })
    }

    pub fn load(cache_dir: &Path) -> Result<PriceCatalog, PricingError> {
        Ok(PriceCatalog {
            supplement: Supplement::parse(BUNDLED_SUPPLEMENT)?,
            litellm: cache::cached_or_bundled(cache_dir, Feed::LiteLlm)?,
            models_dev: cache::cached_or_bundled(cache_dir, Feed::ModelsDev)?,
        })
    }

    #[must_use]
    pub fn resolve(&self, model: &str) -> Option<ResolvedModel> {
        let name = normalize(model);
        stems(&name, self.supplement.strip_suffixes())
            .into_iter()
            .find_map(|stem| {
                let alias = self.supplement.alias(stem);
                alias
                    .and_then(|canonical| self.lookup(canonical))
                    .or_else(|| self.lookup(stem))
            })
    }

    fn lookup(&self, name: &str) -> Option<ResolvedModel> {
        [
            (PriceSource::Supplement, self.supplement.pricing()),
            (PriceSource::LiteLlm, &self.litellm),
            (PriceSource::ModelsDev, &self.models_dev),
        ]
        .into_iter()
        .find_map(|(source, catalog)| {
            catalog.get(name).map(|(key, rates)| ResolvedModel {
                key: key.to_owned(),
                source,
                rates: *rates,
            })
        })
    }

    fn tier_multiplier(&self, resolved: &ResolvedModel, tier: ServiceTier) -> Option<Multiplier> {
        if tier == ServiceTier::Standard {
            return Some(Multiplier::ONE);
        }
        let catalog_fast = match tier {
            ServiceTier::Fast => resolved.rates.fast_multiplier,
            _ => None,
        };
        self.supplement
            .model_tier(strip_date(&resolved.key), tier)
            .or(catalog_fast)
            .or_else(|| self.supplement.vendor_tier(resolved.rates.vendor, tier))
    }

    fn web_search_rate(&self, resolved: &ResolvedModel) -> Option<PicoUsd> {
        resolved
            .rates
            .web_search
            .or_else(|| self.supplement.vendor_web_search(resolved.rates.vendor))
    }
}

impl PriceBook for PriceCatalog {
    fn cost(
        &self,
        model: &str,
        tier: ServiceTier,
        tokens: &TokenCounts,
        web_search: u32,
    ) -> Option<MicroUsd> {
        let resolved = self.resolve(model)?;
        let multiplier = self.tier_multiplier(&resolved, tier)?;
        let search_rate = self.web_search_rate(&resolved);
        resolved
            .rates
            .cost(tokens, multiplier, web_search, search_rate)
    }
}

#[cfg(test)]
#[path = "price_catalog_tests.rs"]
mod tests;
