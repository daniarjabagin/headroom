use super::*;

const SAMPLE: &str = r#"{
  "updated_at": "2026-09-23",
  "vendors": {
    "openai": { "web_search_per_request": 0.01, "tiers": { "priority": 2.0, "fast": 2.0 } },
    "anthropic": { "tiers": {} }
  },
  "pricing": {
    "Custom-Model": {
      "vendor": "anthropic",
      "input_per_million": 4,
      "output_per_million": 20,
      "cache_read_per_million": 0.2,
      "web_search_per_request": 0.02,
      "long_context": { "threshold_tokens": 200000, "input_per_million": 8 }
    }
  },
  "tier_multipliers": { "GPT-5.5": { "priority": 2.5 } },
  "strip_suffixes": ["-high"],
  "aliases": [{ "pattern": "^gpt-5\\.1-codex(?:-max)?$", "canonical": "GPT-5.1" }]
}"#;

#[test]
fn parses_pricing_with_fallbacks_and_long_context() {
    let supplement = Supplement::parse(SAMPLE).unwrap();
    let (key, rates) = supplement.pricing().get("custom-model").unwrap();
    assert_eq!(key, "custom-model");
    assert_eq!(rates.standard.cache_read, PicoUsd(200_000));
    assert_eq!(rates.standard.cache_write_5m, PicoUsd(5_000_000));
    assert_eq!(rates.standard.cache_write_1h, PicoUsd(8_000_000));
    assert_eq!(rates.web_search, Some(PicoUsd(20_000_000_000)));
    let long = rates.long_context.unwrap();
    assert_eq!(long.threshold, Tokens(200_000));
    assert_eq!(long.rates.input, PicoUsd(8_000_000));
    assert_eq!(long.rates.output, PicoUsd(20_000_000));
    assert_eq!(long.rates.cache_read, PicoUsd(800_000));
}

#[test]
fn aliases_match_regex_and_lowercase_canonical() {
    let supplement = Supplement::parse(SAMPLE).unwrap();
    assert_eq!(supplement.alias("gpt-5.1-codex-max"), Some("gpt-5.1"));
    assert_eq!(supplement.alias("gpt-5.1-codex-mini"), None);
}

#[test]
fn tiers_resolve_per_model_then_vendor() {
    let supplement = Supplement::parse(SAMPLE).unwrap();
    let priority = ServiceTier::Priority;
    assert_eq!(
        supplement.model_tier("gpt-5.5", priority),
        Some(Multiplier::from_ppm(2_500_000))
    );
    assert_eq!(supplement.model_tier("gpt-5.5", ServiceTier::Fast), None);
    assert_eq!(
        supplement.vendor_tier(Vendor::OpenAi, ServiceTier::Fast),
        Some(Multiplier::from_ppm(2_000_000))
    );
    assert_eq!(supplement.vendor_tier(Vendor::Anthropic, priority), None);
    assert_eq!(
        supplement.vendor_web_search(Vendor::OpenAi),
        Some(PicoUsd(10_000_000_000))
    );
    assert_eq!(supplement.vendor_web_search(Vendor::Anthropic), None);
}

#[test]
fn invalid_alias_pattern_is_an_error() {
    let text = r#"{ "aliases": [{ "pattern": "(", "canonical": "x" }] }"#;
    assert!(matches!(
        Supplement::parse(text),
        Err(PricingError::AliasPattern { .. })
    ));
}

#[test]
fn pricing_without_output_is_an_error() {
    let text = r#"{ "pricing": { "m": { "vendor": "openai", "input_per_million": 1 } } }"#;
    assert!(matches!(
        Supplement::parse(text),
        Err(PricingError::IncompleteRates { .. })
    ));
}

#[test]
fn unknown_vendor_is_an_error() {
    let text = r#"{ "pricing": { "m": { "vendor": "google", "input_per_million": 1, "output_per_million": 2 } } }"#;
    assert!(matches!(
        Supplement::parse(text),
        Err(PricingError::Json { .. })
    ));
}

#[test]
fn bundled_supplement_parses() {
    let supplement = Supplement::parse(include_str!("../resources/supplement.json")).unwrap();
    assert!(supplement.pricing().get("codex-mini-latest").is_some());
    assert_eq!(supplement.alias("gpt-5.2-codex"), Some("gpt-5.2"));
    let today = "2026-09-23T10:00:00Z".parse().unwrap();
    assert_eq!(
        supplement.dated_alias("codex-auto-review", today),
        Some("gpt-5.5")
    );
}
