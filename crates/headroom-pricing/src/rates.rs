use headroom_core::tokens::TokenCounts;
use headroom_core::units::{MicroUsd, Tokens};
use serde::Serialize;

use crate::money::{Multiplier, PicoUsd, pico_to_atto, pico_to_atto_ppm, round_atto_to_micro};
use crate::source::Vendor;

const ANTHROPIC_CACHE_READ: Multiplier = Multiplier::from_ppm(100_000);
const ANTHROPIC_CACHE_WRITE_5M: Multiplier = Multiplier::from_ppm(1_250_000);
const ANTHROPIC_CACHE_WRITE_1H: Multiplier = Multiplier::from_ppm(2_000_000);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct RateSet {
    pub input: PicoUsd,
    pub output: PicoUsd,
    pub cache_read: PicoUsd,
    pub cache_write_5m: PicoUsd,
    pub cache_write_1h: PicoUsd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct LongContext {
    pub threshold: Tokens,
    pub rates: RateSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct ModelRates {
    pub vendor: Vendor,
    pub standard: RateSet,
    pub long_context: Option<LongContext>,
    pub web_search: Option<PicoUsd>,
    pub fast_multiplier: Option<Multiplier>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RawRates {
    pub(crate) input: Option<PicoUsd>,
    pub(crate) output: Option<PicoUsd>,
    pub(crate) cache_read: Option<PicoUsd>,
    pub(crate) cache_write_5m: Option<PicoUsd>,
    pub(crate) cache_write_1h: Option<PicoUsd>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RawModel {
    pub(crate) vendor: Vendor,
    pub(crate) standard: RawRates,
    pub(crate) long_context: Option<(Tokens, RawRates)>,
    pub(crate) web_search: Option<PicoUsd>,
    pub(crate) fast_multiplier: Option<Multiplier>,
}

impl RawModel {
    pub(crate) fn new(vendor: Vendor) -> RawModel {
        RawModel {
            vendor,
            standard: RawRates::default(),
            long_context: None,
            web_search: None,
            fast_multiplier: None,
        }
    }

    pub(crate) fn complete(&self) -> Option<ModelRates> {
        let standard = complete_set(self.vendor, &self.standard, None)?;
        let long_context = self.long_context.and_then(|(threshold, raw)| {
            complete_set(self.vendor, &raw, Some(&standard))
                .map(|rates| LongContext { threshold, rates })
        });
        Some(ModelRates {
            vendor: self.vendor,
            standard,
            long_context,
            web_search: self.web_search,
            fast_multiplier: self.fast_multiplier,
        })
    }
}

fn complete_set(vendor: Vendor, raw: &RawRates, base: Option<&RateSet>) -> Option<RateSet> {
    let input = raw.input.or(base.map(|b| b.input))?;
    let output = raw.output.or(base.map(|b| b.output))?;
    let short_write = raw.cache_write_5m.unwrap_or_else(|| match vendor {
        Vendor::Anthropic => input.scaled(ANTHROPIC_CACHE_WRITE_5M),
        Vendor::OpenAi => input,
    });
    let long_write = raw.cache_write_1h.unwrap_or_else(|| match vendor {
        Vendor::Anthropic => input.scaled(ANTHROPIC_CACHE_WRITE_1H),
        Vendor::OpenAi => short_write,
    });
    let read = raw.cache_read.unwrap_or_else(|| match vendor {
        Vendor::Anthropic => input.scaled(ANTHROPIC_CACHE_READ),
        Vendor::OpenAi => input,
    });
    Some(RateSet {
        input,
        output,
        cache_read: read,
        cache_write_5m: short_write,
        cache_write_1h: long_write,
    })
}

impl ModelRates {
    #[must_use]
    pub fn rates_for(&self, tokens: &TokenCounts) -> &RateSet {
        match &self.long_context {
            Some(long) if prompt_tokens(tokens) > long.threshold => &long.rates,
            _ => &self.standard,
        }
    }

    pub(crate) fn cost(
        &self,
        tokens: &TokenCounts,
        multiplier: Multiplier,
        searches: u32,
        search_rate: Option<PicoUsd>,
    ) -> Option<MicroUsd> {
        let token_atto = pico_to_atto_ppm(self.rates_for(tokens).token_pico(tokens)?, multiplier)?;
        let search_atto = match (searches, search_rate) {
            (0, _) => 0,
            (count, Some(rate)) => pico_to_atto(u128::from(count) * u128::from(rate.0))?,
            (_, None) => return None,
        };
        round_atto_to_micro(token_atto.checked_add(search_atto)?)
    }
}

impl RateSet {
    fn token_pico(&self, tokens: &TokenCounts) -> Option<u128> {
        [
            (tokens.input, self.input),
            (tokens.output, self.output),
            (tokens.cache_read, self.cache_read),
            (tokens.cache_write_5m, self.cache_write_5m),
            (tokens.cache_write_1h, self.cache_write_1h),
        ]
        .into_iter()
        .try_fold(0u128, |sum, (count, rate)| {
            sum.checked_add(u128::from(count.0) * u128::from(rate.0))
        })
    }
}

fn prompt_tokens(tokens: &TokenCounts) -> Tokens {
    tokens.input + tokens.cache_read + tokens.cache_write_5m + tokens.cache_write_1h
}

#[cfg(test)]
#[path = "rates_tests.rs"]
mod tests;
