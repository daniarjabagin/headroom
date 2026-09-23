use std::collections::BTreeMap;

use crate::error::PricingError;
use crate::normalize::strip_date;
use crate::rates::ModelRates;
use crate::source::PriceSource;

#[derive(Debug, Default)]
pub(crate) struct Catalog {
    entries: BTreeMap<String, ModelRates>,
    undated: BTreeMap<String, String>,
}

impl Catalog {
    pub(crate) fn from_entries(
        feed: PriceSource,
        entries: impl IntoIterator<Item = (String, ModelRates)>,
    ) -> Result<Catalog, PricingError> {
        let catalog = Catalog::from_supplement(entries);
        if catalog.entries.is_empty() {
            return Err(PricingError::EmptyCatalog { feed });
        }
        Ok(catalog)
    }

    pub(crate) fn from_supplement(
        entries: impl IntoIterator<Item = (String, ModelRates)>,
    ) -> Catalog {
        let entries: BTreeMap<String, ModelRates> = entries.into_iter().collect();
        let undated = unambiguous_undated_keys(&entries);
        Catalog { entries, undated }
    }

    pub(crate) fn overlaid_with(self, newer: Catalog) -> Catalog {
        let mut entries = self.entries;
        entries.extend(newer.entries);
        Catalog::from_supplement(entries)
    }

    pub(crate) fn get(&self, name: &str) -> Option<(&str, &ModelRates)> {
        self.entries
            .get_key_value(name)
            .or_else(|| {
                let key = self.undated.get(name)?;
                self.entries.get_key_value(key)
            })
            .map(|(key, rates)| (key.as_str(), rates))
    }
}

fn unambiguous_undated_keys(entries: &BTreeMap<String, ModelRates>) -> BTreeMap<String, String> {
    let mut groups: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for key in entries.keys() {
        let stem = strip_date(key);
        if stem != key && !entries.contains_key(stem) {
            groups.entry(stem).or_default().push(key);
        }
    }
    groups
        .into_iter()
        .filter_map(|(stem, keys)| {
            let first = entries.get(*keys.first()?)?;
            let same_price = keys.iter().all(|key| entries.get(*key) == Some(first));
            let latest = keys.last()?;
            same_price.then(|| (stem.to_owned(), (*latest).to_owned()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::PicoUsd;
    use crate::rates::{RawModel, RawRates};
    use crate::source::Vendor;

    fn rates(input: u64) -> ModelRates {
        let mut raw = RawModel::new(Vendor::Anthropic);
        raw.standard = RawRates {
            input: Some(PicoUsd(input)),
            output: Some(PicoUsd(input * 5)),
            ..RawRates::default()
        };
        raw.complete().unwrap()
    }

    fn catalog(entries: &[(&str, u64)]) -> Catalog {
        Catalog::from_supplement(entries.iter().map(|(k, v)| ((*k).to_owned(), rates(*v))))
    }

    #[test]
    fn exact_key_wins_over_undated_index() {
        let c = catalog(&[("m-1", 1), ("m-1-20250101", 2)]);
        assert_eq!(c.get("m-1").unwrap().0, "m-1");
        assert_eq!(c.get("m-1-20250101").unwrap().1, &rates(2));
    }

    #[test]
    fn undated_name_resolves_only_when_all_dated_variants_agree() {
        let same = catalog(&[("m-20250101", 1), ("m-20250601", 1)]);
        assert_eq!(same.get("m").unwrap().0, "m-20250601");
        let differ = catalog(&[("m-20250101", 1), ("m-2025-06-01", 2)]);
        assert!(differ.get("m").is_none());
    }

    #[test]
    fn empty_feed_is_an_error() {
        let result = Catalog::from_entries(PriceSource::LiteLlm, Vec::new());
        assert!(matches!(result, Err(PricingError::EmptyCatalog { .. })));
    }
}
