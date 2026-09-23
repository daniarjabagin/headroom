use super::*;

fn sonnet_like() -> ModelRates {
    let mut raw = RawModel::new(Vendor::Anthropic);
    raw.standard = RawRates {
        input: Some(PicoUsd(3_000_000)),
        output: Some(PicoUsd(15_000_000)),
        cache_read: Some(PicoUsd(300_000)),
        cache_write_5m: Some(PicoUsd(3_750_000)),
        cache_write_1h: Some(PicoUsd(6_000_000)),
    };
    raw.long_context = Some((
        Tokens(200_000),
        RawRates {
            input: Some(PicoUsd(6_000_000)),
            output: Some(PicoUsd(22_500_000)),
            cache_read: Some(PicoUsd(600_000)),
            cache_write_5m: Some(PicoUsd(7_500_000)),
            cache_write_1h: Some(PicoUsd(12_000_000)),
        },
    ));
    raw.complete().unwrap()
}

fn without_long_context() -> ModelRates {
    ModelRates {
        long_context: None,
        ..sonnet_like()
    }
}

fn tokens(
    input: u64,
    cache_read: u64,
    five_minute: u64,
    one_hour: u64,
    output: u64,
) -> TokenCounts {
    TokenCounts {
        input: Tokens(input),
        cache_read: Tokens(cache_read),
        cache_write_5m: Tokens(five_minute),
        cache_write_1h: Tokens(one_hour),
        output: Tokens(output),
        reasoning: Tokens(output / 2),
    }
}

fn standard_cost(rates: &ModelRates, counts: &TokenCounts) -> Option<MicroUsd> {
    rates.cost(counts, Multiplier::ONE, 0, None)
}

#[test]
fn each_token_class_uses_its_own_rate() {
    let rates = without_long_context();
    let cases = [
        ("input", tokens(1_000_000, 0, 0, 0, 0), 3_000_000),
        ("output", tokens(0, 0, 0, 0, 1_000), 15_000),
        ("cache read", tokens(0, 10_000, 0, 0, 0), 3_000),
        ("cache write 5m", tokens(0, 0, 10_000, 0, 0), 37_500),
        ("cache write 1h", tokens(0, 0, 0, 10_000, 0), 60_000),
        ("empty", tokens(0, 0, 0, 0, 0), 0),
        ("mixed", tokens(100_000, 40_000, 60_000, 0, 20_000), 837_000),
    ];
    for (name, counts, expected) in cases {
        assert_eq!(
            standard_cost(&rates, &counts),
            Some(MicroUsd(expected)),
            "{name}"
        );
    }
}

#[test]
fn long_context_applies_to_whole_request_only_above_threshold() {
    let rates = sonnet_like();
    let at_threshold = tokens(100_000, 40_000, 60_000, 0, 20_000);
    let above = tokens(100_000, 40_001, 60_000, 0, 20_000);
    assert_eq!(
        standard_cost(&rates, &at_threshold),
        Some(MicroUsd(837_000))
    );
    assert_eq!(standard_cost(&rates, &above), Some(MicroUsd(1_524_001)));
    assert_eq!(rates.rates_for(&above), &rates.long_context.unwrap().rates);
}

#[test]
fn long_context_counts_one_hour_writes_toward_prompt() {
    let rates = sonnet_like();
    let counts = tokens(0, 0, 0, 200_001, 0);
    assert_eq!(standard_cost(&rates, &counts), Some(MicroUsd(2_400_012)));
}

#[test]
fn large_output_does_not_trigger_long_context() {
    let rates = sonnet_like();
    let counts = tokens(10, 0, 0, 0, 300_000);
    assert_eq!(standard_cost(&rates, &counts), Some(MicroUsd(4_500_030)));
}

#[test]
fn tier_multiplier_scales_tokens_but_not_web_search() {
    let rates = without_long_context();
    let counts = tokens(1_000_000, 0, 0, 0, 100_000);
    let factor = Multiplier::from_ppm(2_500_000);
    let search = Some(PicoUsd(10_000_000_000));
    assert_eq!(
        rates.cost(&counts, factor, 0, None),
        Some(MicroUsd(11_250_000))
    );
    assert_eq!(
        rates.cost(&counts, factor, 3, search),
        Some(MicroUsd(11_280_000))
    );
    assert_eq!(
        rates.cost(&counts, Multiplier::ONE, 3, search),
        Some(MicroUsd(4_530_000))
    );
}

#[test]
fn web_search_without_a_price_is_unpriced() {
    let rates = sonnet_like();
    let counts = tokens(1, 0, 0, 0, 0);
    assert_eq!(rates.cost(&counts, Multiplier::ONE, 1, None), None);
    assert_eq!(
        rates.cost(&counts, Multiplier::ONE, 0, None),
        Some(MicroUsd(3))
    );
}

#[test]
fn anthropic_missing_cache_classes_follow_published_multipliers() {
    let mut raw = RawModel::new(Vendor::Anthropic);
    raw.standard.input = Some(PicoUsd(5_000_000));
    raw.standard.output = Some(PicoUsd(25_000_000));
    let rates = raw.complete().unwrap().standard;
    assert_eq!(rates.cache_read, PicoUsd(500_000));
    assert_eq!(rates.cache_write_5m, PicoUsd(6_250_000));
    assert_eq!(rates.cache_write_1h, PicoUsd(10_000_000));
}

#[test]
fn openai_missing_cache_classes_bill_at_input() {
    let mut raw = RawModel::new(Vendor::OpenAi);
    raw.standard.input = Some(PicoUsd(30_000_000));
    raw.standard.output = Some(PicoUsd(180_000_000));
    let rates = raw.complete().unwrap().standard;
    assert_eq!(rates.cache_read, PicoUsd(30_000_000));
    assert_eq!(rates.cache_write_5m, PicoUsd(30_000_000));
    assert_eq!(rates.cache_write_1h, PicoUsd(30_000_000));
}

#[test]
fn explicit_cache_write_5m_does_not_leak_into_1h_for_anthropic() {
    let mut raw = RawModel::new(Vendor::Anthropic);
    raw.standard.input = Some(PicoUsd(2_000_000));
    raw.standard.output = Some(PicoUsd(10_000_000));
    raw.standard.cache_write_5m = Some(PicoUsd(2_600_000));
    let rates = raw.complete().unwrap().standard;
    assert_eq!(rates.cache_write_5m, PicoUsd(2_600_000));
    assert_eq!(rates.cache_write_1h, PicoUsd(4_000_000));
}

#[test]
fn long_context_missing_classes_derive_from_long_input_or_standard_output() {
    let mut raw = RawModel::new(Vendor::Anthropic);
    raw.standard.input = Some(PicoUsd(3_000_000));
    raw.standard.output = Some(PicoUsd(15_000_000));
    raw.long_context = Some((
        Tokens(200_000),
        RawRates {
            input: Some(PicoUsd(6_000_000)),
            ..RawRates::default()
        },
    ));
    let long = raw.complete().unwrap().long_context.unwrap().rates;
    assert_eq!(long.output, PicoUsd(15_000_000));
    assert_eq!(long.cache_read, PicoUsd(600_000));
    assert_eq!(long.cache_write_5m, PicoUsd(7_500_000));
    assert_eq!(long.cache_write_1h, PicoUsd(12_000_000));
}

#[test]
fn missing_input_or_output_leaves_model_unpriced() {
    let mut raw = RawModel::new(Vendor::OpenAi);
    raw.standard.input = Some(PicoUsd(1));
    assert!(raw.complete().is_none());
}

#[test]
fn each_event_rounds_once_half_up() {
    let mut raw = RawModel::new(Vendor::OpenAi);
    raw.standard.input = Some(PicoUsd(300_000));
    raw.standard.output = Some(PicoUsd(300_000));
    let rates = raw.complete().unwrap();
    let cases = [(1, 0), (2, 1), (5, 2), (4, 1), (1_000, 300)];
    for (input, expected) in cases {
        let counts = tokens(input, 0, 0, 0, 0);
        assert_eq!(
            standard_cost(&rates, &counts),
            Some(MicroUsd(expected)),
            "{input}"
        );
    }
    let split = tokens(1, 0, 0, 0, 1);
    assert_eq!(standard_cost(&rates, &split), Some(MicroUsd(1)));
}

#[test]
fn many_small_events_stay_within_half_a_micro_each_of_one_big_event() {
    let rates = without_long_context();
    let small = tokens(1_237, 5_011, 713, 97, 389);
    let events: u64 = 1_000;
    let summed: i64 = (0..events)
        .map(|_| standard_cost(&rates, &small).unwrap().0)
        .sum();
    let big = tokens(1_237_000, 5_011_000, 713_000, 97_000, 389_000);
    let whole = standard_cost(&rates, &big).unwrap().0;
    let bound = i64::try_from(events / 2).unwrap();
    assert!((summed - whole).abs() <= bound, "{summed} vs {whole}");
}

#[test]
fn whole_micro_events_sum_exactly() {
    let rates = sonnet_like();
    let small = tokens(1_000, 1_000, 1_000, 1_000, 1_000);
    let summed: i64 = (0..50)
        .map(|_| standard_cost(&rates, &small).unwrap().0)
        .sum();
    let big = tokens(50_000, 50_000, 50_000, 50_000, 50_000);
    assert_eq!(summed, 28_050 * 50);
    assert_eq!(standard_cost(&rates, &big), Some(MicroUsd(28_050 * 50)));
}
