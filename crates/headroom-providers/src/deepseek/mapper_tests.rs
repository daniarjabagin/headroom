use super::*;

const BALANCE: &str = include_str!("fixtures/balance.json");
const TWO_CURRENCIES: &str = include_str!("fixtures/balance_two_currencies.json");
const UNAVAILABLE: &str = include_str!("fixtures/balance_unavailable.json");

fn raw(body: &str) -> RawBalance {
    serde_json::from_str(body).unwrap()
}

fn money(id: &str, currency: &str, micros: i64) -> Balance {
    Balance {
        id: id.into(),
        label: "Balance".into(),
        amount: BalanceAmount::Money(Money {
            currency: CurrencyCode::parse(currency).unwrap(),
            micros,
        }),
    }
}

fn with_totals(available: bool, totals: &[(&str, &str)]) -> RawBalance {
    let infos: Vec<String> = totals
        .iter()
        .map(|(currency, total)| {
            format!(r#"{{"currency":"{currency}","total_balance":"{total}"}}"#)
        })
        .collect();
    raw(&format!(
        r#"{{"is_available":{available},"balance_infos":[{}]}}"#,
        infos.join(",")
    ))
}

#[test]
fn the_documented_example_is_one_yuan_balance() {
    let mapped = map(&raw(BALANCE)).unwrap();
    assert_eq!(mapped.balances, [money("balance_cny", "CNY", 110_000_000)]);
    assert_eq!(mapped.notices, []);
}

#[test]
fn each_currency_is_its_own_balance_and_never_summed() {
    let mapped = map(&raw(TWO_CURRENCIES)).unwrap();
    assert_eq!(
        mapped.balances,
        [
            money("balance_cny", "CNY", 57_341_984),
            money("balance_usd", "USD", 4_250_000),
        ]
    );
}

#[test]
fn an_empty_unavailable_balance_is_critical() {
    let mapped = map(&raw(UNAVAILABLE)).unwrap();
    assert_eq!(mapped.balances, [money("balance_cny", "CNY", 0)]);
    assert_eq!(
        mapped.notices,
        [Notice {
            tone: Tone::Critical,
            text: USED_UP_NOTICE.into(),
        }]
    );
    let nothing = map(&with_totals(false, &[])).unwrap();
    assert_eq!(nothing.notices[0].tone, Tone::Critical);
}

#[test]
fn money_left_but_unavailable_is_a_warning() {
    let mapped = map(&with_totals(false, &[("CNY", "0.50"), ("USD", "-1.00")])).unwrap();
    assert_eq!(
        mapped.notices,
        [Notice {
            tone: Tone::Warning,
            text: UNAVAILABLE_NOTICE.into(),
        }]
    );
    assert_eq!(mapped.balances[1], money("balance_usd", "USD", -1_000_000));
}

#[test]
fn a_negative_balance_is_kept_exactly() {
    let mapped = map(&with_totals(false, &[("CNY", "-0.0000015")])).unwrap();
    assert_eq!(mapped.balances, [money("balance_cny", "CNY", -2)]);
    assert_eq!(mapped.notices[0].tone, Tone::Critical);
}

#[test]
fn lower_case_currencies_are_normalised() {
    let mapped = map(&with_totals(true, &[("cny", "1")])).unwrap();
    assert_eq!(mapped.balances, [money("balance_cny", "CNY", 1_000_000)]);
}

#[test]
fn unknown_or_repeated_currencies_are_invalid() {
    let bad = map(&with_totals(true, &[("yuan", "1")])).unwrap_err();
    assert_eq!(
        bad,
        ProviderError::InvalidResponse(
            "DeepSeek balance: \"yuan\" is not an ISO 4217 currency code".into()
        )
    );
    let twice = map(&with_totals(true, &[("CNY", "1"), ("cny", "2")])).unwrap_err();
    assert_eq!(
        twice,
        ProviderError::InvalidResponse("DeepSeek listed the balance_cny balance twice".into())
    );
}
