use super::*;

fn sample() -> Value {
    json!({
        "openai": {
            "id": "openai",
            "name": "OpenAI",
            "models": {
                "gpt-5.4": {
                    "id": "gpt-5.4",
                    "cost": {
                        "input": 2.5, "output": 15, "cache_read": 0.25,
                        "tiers": [{ "input": 5, "output": 22.5, "cache_read": 0.5, "tier": { "type": "context", "size": 272_000 } }],
                        "context_over_200k": { "input": 5, "output": 22.5, "cache_read": 0.5 }
                    }
                },
                "gpt-5-pro": { "cost": { "input": 15, "output": 120 } },
                "whisper-1": { "id": "whisper-1" }
            }
        },
        "anthropic": {
            "models": {
                "claude-sonnet-4": {
                    "cost": {
                        "input": 3, "output": 15, "cache_read": 0.3, "cache_write": 3.75,
                        "context_over_200k": { "input": 6, "output": 22.5, "cache_read": 0.6, "cache_write": 7.5 }
                    }
                }
            }
        },
        "somebody-else": {
            "models": { "gpt-5.4": { "cost": { "input": 99, "output": 99 } } }
        }
    })
}

#[test]
fn context_tier_beats_legacy_over_200k_field() {
    let catalog = parse(&sample()).unwrap();
    let (_, rates) = catalog.get("gpt-5.4").unwrap();
    assert_eq!(rates.vendor, Vendor::OpenAi);
    assert_eq!(rates.standard.input, PicoUsd(2_500_000));
    let long = rates.long_context.unwrap();
    assert_eq!(long.threshold, Tokens(272_000));
    assert_eq!(long.rates.cache_read, PicoUsd(500_000));
}

#[test]
fn legacy_over_200k_field_sets_a_200k_threshold() {
    let catalog = parse(&sample()).unwrap();
    let (_, rates) = catalog.get("claude-sonnet-4").unwrap();
    let long = rates.long_context.unwrap();
    assert_eq!(long.threshold, Tokens(200_000));
    assert_eq!(long.rates.cache_write_5m, PicoUsd(7_500_000));
    assert_eq!(long.rates.cache_write_1h, PicoUsd(12_000_000));
    assert_eq!(rates.standard.cache_write_1h, PicoUsd(6_000_000));
    assert_eq!(rates.web_search, None);
}

#[test]
fn missing_cache_prices_follow_vendor_rules() {
    let catalog = parse(&sample()).unwrap();
    let (_, rates) = catalog.get("gpt-5-pro").unwrap();
    assert_eq!(rates.standard.cache_read, PicoUsd(15_000_000));
}

#[test]
fn only_openai_and_anthropic_providers_are_read() {
    let catalog = parse(&sample()).unwrap();
    assert_eq!(
        catalog.get("gpt-5.4").unwrap().1.standard.input,
        PicoUsd(2_500_000)
    );
    assert!(catalog.get("whisper-1").is_none());
}

#[test]
fn trim_is_idempotent_and_preserves_prices() {
    let trimmed = trim(&sample());
    assert!(trimmed.get("somebody-else").is_none());
    assert!(trimmed["openai"]["models"].get("whisper-1").is_none());
    assert_eq!(trim(&trimmed), trimmed);
    let from_trimmed = parse(&trimmed).unwrap();
    let from_full = parse(&sample()).unwrap();
    assert_eq!(
        from_trimmed.get("claude-sonnet-4"),
        from_full.get("claude-sonnet-4")
    );
}

#[test]
fn bundled_snapshot_is_already_trimmed() {
    let bundled: Value =
        serde_json::from_str(include_str!("../resources/models_dev.json")).unwrap();
    assert_eq!(trim(&bundled), bundled);
}

#[test]
fn document_without_vendors_is_rejected() {
    assert!(matches!(
        parse(&json!({ "other": {} })),
        Err(PricingError::EmptyCatalog { .. })
    ));
}
