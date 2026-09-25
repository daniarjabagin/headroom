from mock_accounts import account, pace, showcase_accounts, window
from mock_common import DAY, HOUR
from mock_state import assemble, showcase_usage


def money(balance_id, label, currency, micros):
    return {"id": balance_id, "label": label, "kind": "money", "currency": currency, "micros": micros}


def count(balance_id, label, value, unit):
    return {"id": balance_id, "label": label, "kind": "count", "value": value, "unit": unit}


def kilo(now):
    return account(
        "kilo:2c1b0a9f8e7d", "kilo", None, "dev@example.com", None, "fresh", [], now,
        owner="headroom",
        balances=[{"id": "credits", "label": "Credit balance", "kind": "usd", "usd_micros": 18_123_456}],
    )


def warp(now):
    return account(
        "warp:8e7d6c5b4a3f", "warp", None, "dev@example.com", "Build", "fresh",
        [window("monthly", "Monthly credits", 37.0, 19 * DAY + 6 * HOUR, 30 * DAY, "good",
                pace("healthy", 36.0, 91.0), now)],
        now,
        owner="headroom",
        balances=[count("bonus_credits", "Bonus credits", 250, "credits")],
    )


def poe(now):
    return account(
        "poe:5a4f3e2d1c0b", "poe", None, None, None, "fresh", [], now,
        owner="headroom",
        balances=[count("points", "Point balance", 1_184_250, "points")],
    )


def deepseek(now):
    return account(
        "deepseek:0b9a8f7e6d5c", "deepseek", None, None, None, "fresh", [], now,
        owner="headroom",
        balances=[money("balance_cny", "Balance", "CNY", 57_341_984),
                  money("balance_usd", "Balance", "USD", 12_500_000)],
    )


def moonshot(now):
    return account(
        "moonshot:7f6e5d4c3b2a", "moonshot", None, None, None, "fresh", [], now,
        owner="headroom",
        balances=[money("available", "Balance", "CNY", -3_000_000),
                  money("voucher", "Vouchers", "CNY", 0),
                  money("cash", "Cash", "CNY", -3_000_000)],
        notices=[{"tone": "critical", "text": "Balance is used up; API requests fail until you top up"},
                 {"tone": "critical", "text": "Cash balance is negative: the account is in debt"}],
    )


def providers_state(now):
    claude = showcase_accounts(now)[0]
    accounts = [claude, kilo(now), warp(now), poe(now), deepseek(now), moonshot(now)]
    return assemble(now, accounts, showcase_usage(now)[:1], ("claude:0a1b2c3d4e5f", "weekly"))
