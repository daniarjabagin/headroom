use serde_json::json;

use super::*;

fn sample() -> Value {
    json!({
        "sample_spec": { "input_cost_per_token": 0.0, "litellm_provider": "one of …", "mode": "one of …" },
        "claude-sonnet-4-5": {
            "litellm_provider": "anthropic",
            "mode": "chat",
            "max_tokens": 64000,
            "input_cost_per_token": 3e-06,
            "output_cost_per_token": 1.5e-05,
            "cache_read_input_token_cost": 3e-07,
            "cache_creation_input_token_cost": 3.75e-06,
            "cache_creation_input_token_cost_above_1hr": 6e-06,
            "input_cost_per_token_above_200k_tokens": 6e-06,
            "output_cost_per_token_above_200k_tokens": 2.25e-05,
            "cache_read_input_token_cost_above_200k_tokens": 6e-07,
            "cache_creation_input_token_cost_above_200k_tokens": 7.5e-06,
            "cache_creation_input_token_cost_above_1hr_above_200k_tokens": 1.2e-05,
            "search_context_cost_per_query": { "search_context_size_medium": 0.01 },
            "provider_specific_entry": { "us": 1.1, "fast": 2.0 }
        },
        "gpt-5.5": {
            "litellm_provider": "openai",
            "mode": "responses",
            "input_cost_per_token": 5e-06,
            "output_cost_per_token": 3e-05,
            "cache_read_input_token_cost": 5e-07,
            "input_cost_per_token_priority": 1.25e-05,
            "input_cost_per_token_above_272k_tokens": 1e-05,
            "input_cost_per_token_above_272k_tokens_priority": 2.5e-05,
            "output_cost_per_token_above_272k_tokens": 4.5e-05
        },
        "bedrock/claude-sonnet-4-5": {
            "litellm_provider": "bedrock",
            "mode": "chat",
            "input_cost_per_token": 3.3e-06,
            "output_cost_per_token": 1.65e-05
        },
        "text-embedding-3-small": {
            "litellm_provider": "openai",
            "mode": "embedding",
            "input_cost_per_token": 2e-08,
            "output_cost_per_token": 0.0
        },
        "ft:gpt-4o-2024-08-06": {
            "litellm_provider": "openai",
            "mode": "chat",
            "input_cost_per_token": 3.75e-06,
            "output_cost_per_token": 1.5e-05
        }
    })
}

#[test]
fn parses_all_classes_long_context_web_search_and_fast() {
    let catalog = parse(&sample()).unwrap();
    let (_, rates) = catalog.get("claude-sonnet-4-5").unwrap();
    assert_eq!(rates.vendor, Vendor::Anthropic);
    assert_eq!(rates.standard.input, PicoUsd(3_000_000));
    assert_eq!(rates.standard.cache_write_1h, PicoUsd(6_000_000));
    let long = rates.long_context.unwrap();
    assert_eq!(long.threshold, Tokens(200_000));
    assert_eq!(long.rates.output, PicoUsd(22_500_000));
    assert_eq!(long.rates.cache_write_1h, PicoUsd(12_000_000));
    assert_eq!(rates.web_search, Some(PicoUsd(10_000_000_000)));
    assert_eq!(rates.fast_multiplier, Some(Multiplier::from_ppm(2_000_000)));
}

#[test]
fn openai_threshold_comes_from_field_name_and_tier_variants_are_ignored() {
    let catalog = parse(&sample()).unwrap();
    let (_, rates) = catalog.get("gpt-5.5").unwrap();
    let long = rates.long_context.unwrap();
    assert_eq!(long.threshold, Tokens(272_000));
    assert_eq!(long.rates.input, PicoUsd(10_000_000));
    assert_eq!(long.rates.cache_read, PicoUsd(10_000_000));
    assert_eq!(rates.standard.input, PicoUsd(5_000_000));
    assert_eq!(rates.fast_multiplier, None);
}

#[test]
fn other_providers_modes_and_fine_tunes_are_skipped() {
    let catalog = parse(&sample()).unwrap();
    for key in [
        "bedrock/claude-sonnet-4-5",
        "text-embedding-3-small",
        "ft:gpt-4o-2024-08-06",
        "sample_spec",
    ] {
        assert!(catalog.get(key).is_none(), "{key}");
    }
}

#[test]
fn lowest_long_context_threshold_wins() {
    let document = json!({
        "m": {
            "litellm_provider": "openai",
            "mode": "chat",
            "input_cost_per_token": 1e-06,
            "output_cost_per_token": 2e-06,
            "input_cost_per_token_above_272k_tokens": 3e-06,
            "input_cost_per_token_above_128k_tokens": 4e-06,
            "output_cost_per_token_above_272k_tokens": 5e-06
        }
    });
    let catalog = parse(&document).unwrap();
    let long = catalog.get("m").unwrap().1.long_context.unwrap();
    assert_eq!(long.threshold, Tokens(128_000));
    assert_eq!(long.rates.input, PicoUsd(4_000_000));
    assert_eq!(long.rates.output, PicoUsd(2_000_000));
}

#[test]
fn document_without_usable_models_is_rejected() {
    let result = parse(&json!({ "sample_spec": {} }));
    assert!(matches!(result, Err(PricingError::EmptyCatalog { .. })));
    assert!(parse(&json!([1, 2])).is_err());
}

#[test]
fn trim_keeps_only_priced_vendor_entries_and_rate_fields() {
    let trimmed = trim(&sample());
    let keys: Vec<&String> = trimmed.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["claude-sonnet-4-5", "gpt-5.5"]);
    let sonnet = &trimmed["claude-sonnet-4-5"];
    assert!(sonnet.get("max_tokens").is_none());
    assert!(
        sonnet
            .get("cache_creation_input_token_cost_above_1hr_above_200k_tokens")
            .is_some()
    );
    assert!(
        trimmed["gpt-5.5"]
            .get("input_cost_per_token_priority")
            .is_none()
    );
    assert_eq!(trim(&trimmed), trimmed);
    assert_eq!(
        parse(&trimmed).unwrap().get("gpt-5.5"),
        parse(&sample()).unwrap().get("gpt-5.5")
    );
}

#[test]
fn bundled_snapshot_is_already_trimmed() {
    let bundled: Value = serde_json::from_str(include_str!("../resources/litellm.json")).unwrap();
    assert_eq!(trim(&bundled), bundled);
}
