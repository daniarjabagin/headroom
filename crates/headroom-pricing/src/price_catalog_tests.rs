use headroom_core::units::Tokens;
use serde_json::json;

use super::*;

fn bundled() -> PriceCatalog {
    PriceCatalog::bundled().unwrap()
}

fn io(input: u64, output: u64) -> TokenCounts {
    TokenCounts {
        input: Tokens(input),
        output: Tokens(output),
        ..TokenCounts::default()
    }
}

fn resolved(catalog: &PriceCatalog, model: &str) -> (String, PriceSource) {
    let found = catalog
        .resolve(model)
        .unwrap_or_else(|| panic!("{model} unpriced"));
    (found.key, found.source)
}

#[test]
fn bundled_prices_current_claude_models() {
    let catalog = bundled();
    let cases = [
        (
            "claude-opus-5-5",
            "claude-opus-5-5",
            4_000_000,
            20_000_000,
            200_000,
        ),
        (
            "claude-opus-5",
            "claude-opus-5",
            5_000_000,
            25_000_000,
            500_000,
        ),
        (
            "claude-sonnet-5",
            "claude-sonnet-5",
            2_000_000,
            10_000_000,
            200_000,
        ),
        (
            "claude-haiku-4-5-20251001",
            "claude-haiku-4-5-20251001",
            1_000_000,
            5_000_000,
            100_000,
        ),
        (
            "claude-fable-5-1",
            "claude-fable-5-1",
            10_000_000,
            50_000_000,
            250_000,
        ),
        (
            "claude-fable-5",
            "claude-fable-5",
            10_000_000,
            50_000_000,
            1_000_000,
        ),
        (
            "claude-opus-4-8",
            "claude-opus-4-8",
            5_000_000,
            25_000_000,
            500_000,
        ),
    ];
    for (model, key, input, output, cache_read) in cases {
        let found = catalog.resolve(model).unwrap();
        assert_eq!(found.key, key, "{model}");
        assert_eq!(found.source, PriceSource::LiteLlm, "{model}");
        let rates = found.rates.standard;
        assert_eq!(
            (rates.input.0, rates.output.0, rates.cache_read.0),
            (input, output, cache_read),
            "{model}"
        );
        assert_eq!(rates.cache_write_5m.0 * 4, input * 5, "{model}");
        assert_eq!(rates.cache_write_1h.0, input * 2, "{model}");
    }
}

#[test]
fn bundled_prices_codex_models_from_the_right_source() {
    let catalog = bundled();
    let cases = [
        ("gpt-5", "gpt-5", PriceSource::LiteLlm),
        ("gpt-5-codex", "gpt-5", PriceSource::LiteLlm),
        ("gpt-5.1-codex", "gpt-5.1", PriceSource::LiteLlm),
        ("gpt-5.1-codex-max", "gpt-5.1", PriceSource::LiteLlm),
        ("gpt-5.1-codex-mini", "gpt-5-mini", PriceSource::LiteLlm),
        ("gpt-5.2-codex", "gpt-5.2", PriceSource::LiteLlm),
        ("gpt-5.3-codex", "gpt-5.3-codex", PriceSource::LiteLlm),
        (
            "gpt-5.3-codex-spark",
            "gpt-5.3-codex-spark",
            PriceSource::ModelsDev,
        ),
        ("gpt-5.4", "gpt-5.4", PriceSource::LiteLlm),
        ("gpt-5.5", "gpt-5.5", PriceSource::LiteLlm),
        ("gpt-5.5-high", "gpt-5.5", PriceSource::LiteLlm),
        (
            "gpt-5.5-2026-04-23",
            "gpt-5.5-2026-04-23",
            PriceSource::LiteLlm,
        ),
        ("gpt-5.6-sol", "gpt-5.6-sol", PriceSource::LiteLlm),
        (
            "codex-mini-latest",
            "codex-mini-latest",
            PriceSource::Supplement,
        ),
    ];
    for (model, key, source) in cases {
        assert_eq!(
            resolved(&catalog, model),
            (key.to_owned(), source),
            "{model}"
        );
    }
}

#[test]
fn normalization_reaches_catalog_keys() {
    let catalog = bundled();
    let cases = [
        (
            "anthropic/claude-sonnet-4-5-20250929",
            "claude-sonnet-4-5-20250929",
        ),
        ("Claude-Opus-5[1m]", "claude-opus-5"),
        ("claude-haiku-4-5@20251001", "claude-haiku-4-5"),
        ("openai/gpt-5.5", "gpt-5.5"),
        ("claude-opus-5-thinking-xhigh", "claude-opus-5"),
        ("claude-opus-4-6-20260205", "claude-opus-4-6-20260205"),
    ];
    for (model, key) in cases {
        assert_eq!(resolved(&catalog, model).0, key, "{model}");
    }
}

#[test]
fn unknown_and_ambiguous_models_are_unpriced() {
    let catalog = bundled();
    for model in [
        "codex-auto-review",
        "gpt-5.5-mini",
        "mystery",
        "",
        "claude-opus",
        "gpt-5.5-fast",
    ] {
        assert!(catalog.resolve(model).is_none(), "{model}");
        assert_eq!(
            catalog.cost(model, ServiceTier::Standard, &io(1, 1), 0),
            None,
            "{model}"
        );
    }
}

#[test]
fn tier_multipliers_come_from_supplement_catalog_then_vendor_default() {
    let catalog = bundled();
    let tokens = io(100_000, 10_000);
    let cases = [
        ("gpt-5.5", ServiceTier::Standard, Some(800_000)),
        ("gpt-5.5", ServiceTier::Priority, Some(2_000_000)),
        ("gpt-5.5-2026-04-23", ServiceTier::Fast, Some(2_000_000)),
        ("gpt-5.4", ServiceTier::Priority, Some(800_000)),
        ("gpt-5-mini", ServiceTier::Priority, Some(81_000)),
        ("claude-opus-5", ServiceTier::Fast, Some(1_500_000)),
        ("claude-opus-5-5", ServiceTier::Fast, Some(1_200_000)),
        ("claude-opus-4-6", ServiceTier::Fast, Some(4_500_000)),
        ("claude-sonnet-5", ServiceTier::Fast, None),
        ("claude-opus-5", ServiceTier::Priority, None),
    ];
    for (model, tier, expected) in cases {
        assert_eq!(
            catalog.cost(model, tier, &tokens, 0),
            expected.map(MicroUsd),
            "{model} {tier:?}"
        );
    }
}

#[test]
fn openai_long_context_uses_272k_threshold() {
    let catalog = bundled();
    let standard = ServiceTier::Standard;
    assert_eq!(
        catalog.cost("gpt-5.5", standard, &io(272_000, 0), 0),
        Some(MicroUsd(1_360_000))
    );
    assert_eq!(
        catalog.cost("gpt-5.5", standard, &io(272_001, 0), 0),
        Some(MicroUsd(2_720_010))
    );
}

#[test]
fn current_claude_models_have_no_long_context_premium() {
    let catalog = bundled();
    let big = io(900_000, 0);
    assert_eq!(
        catalog.cost("claude-opus-5", ServiceTier::Standard, &big, 0),
        Some(MicroUsd(4_500_000))
    );
    let sonnet_45 = catalog.resolve("claude-sonnet-4-5").unwrap().rates;
    assert_eq!(sonnet_45.long_context.unwrap().threshold, Tokens(200_000));
}

#[test]
fn web_search_uses_model_price_then_vendor_default() {
    let catalog = bundled();
    let standard = ServiceTier::Standard;
    assert_eq!(
        catalog.cost("claude-sonnet-5", standard, &io(0, 0), 2),
        Some(MicroUsd(20_000))
    );
    assert_eq!(
        catalog.cost("gpt-5.3-codex-spark", standard, &io(0, 0), 1),
        Some(MicroUsd(10_000))
    );
    assert_eq!(
        catalog.cost("claude-sonnet-5", ServiceTier::Priority, &io(0, 0), 1),
        None
    );
}

#[test]
fn cache_writes_price_5m_and_1h_separately() {
    let catalog = bundled();
    let tokens = TokenCounts {
        cache_write_5m: Tokens(100_000),
        cache_write_1h: Tokens(100_000),
        cache_read: Tokens(100_000),
        ..TokenCounts::default()
    };
    assert_eq!(
        catalog.cost("claude-sonnet-5", ServiceTier::Standard, &tokens, 0),
        Some(MicroUsd(250_000 + 400_000 + 20_000))
    );
}

fn custom(supplement: &str) -> PriceCatalog {
    let litellm = json!({
        "gpt-x": { "litellm_provider": "openai", "mode": "chat", "input_cost_per_token": 1e-06, "output_cost_per_token": 2e-06 }
    });
    let models_dev = json!({
        "openai": { "models": {
            "gpt-x": { "cost": { "input": 9, "output": 9 } },
            "gpt-y": { "cost": { "input": 3, "output": 4 } }
        } }
    });
    PriceCatalog {
        supplement: Supplement::parse(supplement).unwrap(),
        litellm: crate::litellm::parse(&litellm).unwrap(),
        models_dev: crate::models_dev::parse(&models_dev).unwrap(),
    }
}

#[test]
fn resolution_prefers_supplement_then_litellm_then_models_dev() {
    let plain = custom("{}");
    assert_eq!(
        resolved(&plain, "gpt-x"),
        ("gpt-x".to_owned(), PriceSource::LiteLlm)
    );
    assert_eq!(
        resolved(&plain, "gpt-y"),
        ("gpt-y".to_owned(), PriceSource::ModelsDev)
    );
    let overridden = custom(
        r#"{ "pricing": { "gpt-x": { "vendor": "openai", "input_per_million": 5, "output_per_million": 6 } } }"#,
    );
    let found = overridden.resolve("gpt-x").unwrap();
    assert_eq!(found.source, PriceSource::Supplement);
    assert_eq!(found.rates.standard.input, PicoUsd(5_000_000));
}

#[test]
fn alias_miss_falls_back_to_the_raw_name() {
    let catalog = custom(r#"{ "aliases": [{ "pattern": "^gpt-y$", "canonical": "missing" }] }"#);
    assert_eq!(resolved(&catalog, "gpt-y").0, "gpt-y");
}

#[test]
fn load_without_cache_uses_bundled_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let catalog = PriceCatalog::load(dir.path()).unwrap();
    assert_eq!(
        resolved(&catalog, "claude-opus-5-5").1,
        PriceSource::LiteLlm
    );
}

#[test]
fn load_ignores_corrupt_cache() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("litellm.json"), b"{not json").unwrap();
    std::fs::write(
        dir.path().join("models_dev.json"),
        br#"{"etag":null,"data":{}}"#,
    )
    .unwrap();
    let catalog = PriceCatalog::load(dir.path()).unwrap();
    assert!(catalog.resolve("gpt-5.3-codex-spark").is_some());
    assert!(catalog.resolve("claude-sonnet-5").is_some());
}
