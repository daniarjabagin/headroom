use crate::i18n::Lang;
use crate::numbers::{exact_tokens, money, usd};
use crate::payload::{Balance, BalanceAmount};

const LABEL_TEXTS: [&str; 10] = [
    "Credits",
    "Extra usage",
    "Balance",
    "Vouchers",
    "Cash",
    "Credit balance",
    "Organization credits",
    "Point balance",
    "Bonus credits",
    "Monthly credits",
];

#[must_use]
pub fn label_text(lang: Lang, label: &str) -> String {
    LABEL_TEXTS
        .iter()
        .find(|known| **known == label)
        .map_or_else(|| label.to_owned(), |known| lang.tr(known).to_owned())
}

#[must_use]
pub fn balance_title(lang: Lang, balance: &Balance) -> String {
    label_text(lang, &balance.label)
}

#[must_use]
pub fn balance_value(lang: Lang, balance: &Balance) -> Option<String> {
    Some(match &balance.amount {
        BalanceAmount::Usd { usd_micros } => usd(*usd_micros),
        BalanceAmount::Money { currency, micros } => money(currency, *micros),
        BalanceAmount::Count { value, unit } => format!("{} {unit}", exact_tokens(lang, *value))
            .trim()
            .to_owned(),
        BalanceAmount::Unknown => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn balance(json: &str) -> Balance {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn english_keeps_the_daemon_label() {
        for label in LABEL_TEXTS.iter().copied().chain(["Key limit"]) {
            assert_eq!(label_text(Lang::En, label), label);
        }
    }

    #[test]
    fn russian_translates_the_known_labels() {
        let expected = [
            ("Credits", "Кредиты"),
            ("Credit balance", "Кредиты"),
            ("Organization credits", "Кредиты организации"),
            ("Balance", "Баланс"),
            ("Vouchers", "Ваучеры"),
            ("Cash", "Денежный баланс"),
            ("Point balance", "Баланс баллов"),
            ("Bonus credits", "Бонусные кредиты"),
            ("Monthly credits", "Кредиты на месяц"),
            ("Extra usage", "Доп. использование"),
            ("Key limit", "Key limit"),
        ];
        for (label, russian) in expected {
            assert_eq!(label_text(Lang::Ru, label), russian, "{label}");
        }
    }

    #[test]
    fn kilo_credit_balance_keeps_its_label() {
        let kilo = balance(
            r#"{"id":"credits","label":"Credit balance","kind":"usd","usd_micros":7250000}"#,
        );
        assert_eq!(balance_title(Lang::En, &kilo), "Credit balance");
        assert_eq!(balance_value(Lang::En, &kilo).as_deref(), Some("$7.25"));
    }

    #[test]
    fn money_balances_render_in_their_currency() {
        let cny = balance(
            r#"{"id":"balance_cny","label":"Balance","kind":"money","currency":"CNY","micros":12500000}"#,
        );
        assert_eq!(balance_title(Lang::Ru, &cny), "Баланс");
        assert_eq!(balance_value(Lang::En, &cny).as_deref(), Some("¥12.50"));
        let cash = balance(
            r#"{"id":"cash","label":"Cash","kind":"money","currency":"CNY","micros":-3000000}"#,
        );
        assert_eq!(balance_value(Lang::En, &cash).as_deref(), Some("-¥3.00"));
    }
}
