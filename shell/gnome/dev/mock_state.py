from datetime import datetime, timedelta, timezone

MINUTE = timedelta(minutes=1)
HOUR = timedelta(hours=1)
DAY = timedelta(days=1)
PERIODS = ("today", "yesterday", "last_30_days")
TOP_MODELS = 5
TRACKED = ("healthy", "close")


def iso(moment):
    return moment.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ") if moment else None


def pace(severity, even=None, projected=None, runs_out_at=None):
    return {
        "severity": severity,
        "even_pace_percent": even,
        "projected_percent": projected,
        "spare_percent": 100.0 - projected if severity in TRACKED and projected is not None else None,
        "runs_out_at": iso(runs_out_at),
    }


def window(window_id, label, used, resets_in, period, tone, window_pace, now):
    return {
        "id": window_id,
        "label": label,
        "used_percent": used,
        "remaining_percent": max(0.0, 100.0 - used),
        "resets_at": iso(now + resets_in) if resets_in else None,
        "period_seconds": int(period.total_seconds()),
        "tone": tone,
        "pace": window_pace,
        "hidden": False,
    }


def tokens(total):
    output = total // 20
    cache_read = total // 3
    return {
        "input": total - output - cache_read,
        "cache_read": cache_read,
        "cache_write": 0,
        "output": output,
        "reasoning": output // 4,
        "total": total,
    }


def model_usage(name, total_tokens, cost_micros, partial):
    return {"model": name, "total_tokens": total_tokens, "cost_usd_micros": cost_micros, "partial": partial}


def by_cost(models):
    return sorted(models, key=lambda model: (-model["cost_usd_micros"], -model["total_tokens"], model["model"]))


def split_models(priced_tokens, cost_micros, mix):
    rows = []
    for name, token_share, cost_share in mix:
        rows.append(model_usage(name, priced_tokens * token_share // 100, cost_micros * cost_share // 100, False))
    rows[0]["total_tokens"] += priced_tokens - sum(row["total_tokens"] for row in rows)
    rows[0]["cost_usd_micros"] += cost_micros - sum(row["cost_usd_micros"] for row in rows)
    return [row for row in rows if row["total_tokens"] > 0]


def totals(total_tokens, cost_micros, mix, unpriced=None):
    unpriced_tokens, unpriced_models = unpriced or (0, [])
    models = split_models(total_tokens - unpriced_tokens, cost_micros, mix)
    models += [model_usage(name, unpriced_tokens // len(unpriced_models), 0, True) for name in unpriced_models]
    return {
        "tokens": tokens(total_tokens),
        "cost_usd_micros": cost_micros,
        "partial": unpriced_tokens > 0,
        "unpriced_tokens": unpriced_tokens,
        "unpriced_models": unpriced_models,
        "models": by_cost(models),
    }


def merge_models(groups):
    merged = {}
    for models in groups:
        for model in models:
            entry = merged.setdefault(model["model"], model_usage(model["model"], 0, 0, False))
            entry["total_tokens"] += model["total_tokens"]
            entry["cost_usd_micros"] += model["cost_usd_micros"]
            entry["partial"] = entry["partial"] or model["partial"]
    return by_cost(merged.values())


def models_other(rest):
    if not rest:
        return None
    return {
        "count": len(rest),
        "total_tokens": sum(model["total_tokens"] for model in rest),
        "cost_usd_micros": sum(model["cost_usd_micros"] for model in rest),
        "partial": any(model["partial"] for model in rest),
    }


def with_top_models(entry):
    models = entry["models"]
    return {**entry, "models": models[:TOP_MODELS], "models_other": models_other(models[TOP_MODELS:])}


def published_usage(entry):
    return {**entry, **{period: with_top_models(entry[period]) for period in PERIODS}}


def published_spend(spend):
    return {**spend, "by_provider": [with_top_models(row) for row in spend["by_provider"]]}


def daily(now, seed, scale):
    days = []
    for offset in range(29, -1, -1):
        wave = (seed * (offset + 3) * 7919) % 100
        value = 0 if wave < 18 else wave * scale
        date = (now - offset * DAY).strftime("%Y-%m-%d")
        days.append({"date": date, "total_tokens": value, "cost_usd_micros": value * 3, "partial": False})
    return days


def usage_entry(provider, home, now, seed, periods, scale):
    entry = {"provider": provider, "usage_home": home}
    entry.update(zip(PERIODS, periods))
    entry["daily"] = daily(now, seed, scale)
    entry["models"] = entry["last_30_days"]["models"]
    return entry


def provider_spend(usage, provider, period):
    entries = [entry[period] for entry in usage if entry["provider"] == provider]
    return {
        "provider": provider,
        "cost_usd_micros": sum(t["cost_usd_micros"] for t in entries),
        "total_tokens": sum(t["tokens"]["total"] for t in entries),
        "partial": any(t["partial"] for t in entries),
        "models": merge_models(t["models"] for t in entries),
    }


def period_spend(usage, period):
    providers = sorted({entry["provider"] for entry in usage})
    rows = [provider_spend(usage, provider, period) for provider in providers]
    rows = [row for row in rows if row["cost_usd_micros"] or row["total_tokens"]]
    rows.sort(key=lambda row: (-row["cost_usd_micros"], row["provider"]))
    return {
        "cost_usd_micros": sum(row["cost_usd_micros"] for row in rows),
        "total_tokens": sum(row["total_tokens"] for row in rows),
        "partial": any(row["partial"] for row in rows),
        "by_provider": rows,
    }


def account(account_id, provider, label, email, plan, status, windows, now, **extra):
    base = {
        "id": account_id,
        "provider": provider,
        "label": label,
        "email": email,
        "plan": plan,
        "owner": "cli",
        "hidden": False,
        "status": status,
        "error": None,
        "updated_at": iso(now - (3 * HOUR if status == "stale" else 2 * MINUTE)),
        "source": "live",
        "windows": windows,
        "balances": [],
        "notices": [],
        "usage_home": "~/.codex" if provider == "codex" else "~/.claude",
    }
    base.update(extra)
    return base


def codex_work(now):
    return account(
        "codex:1a2b3c4d5e6f", "codex", "work", "dev@example.com", "Pro", "fresh",
        [
            window("session", "Session", 38.0, 2 * HOUR + 41 * MINUTE, 5 * HOUR, "good",
                   pace("healthy", 46.0, 82.0), now),
            window("weekly", "Weekly", 19.0, 4 * DAY + 6 * HOUR, 7 * DAY, "good", pace("healthy", 38.0, 50.0), now),
        ],
        now,
        balances=[{"id": "credits", "label": "Credits", "kind": "usd", "usd_micros": 12500000}],
    )


def codex_personal(now):
    return account(
        "codex:9f8e7d6c5b4a", "codex", "personal", "me@example.org", "Plus", "stale",
        [
            window("session", "Session", 71.0, HOUR + 12 * MINUTE, 5 * HOUR, "warning",
                   pace("close", 76.0, 96.0), now),
            window("weekly", "Weekly", 83.0, 3 * DAY + 2 * HOUR, 7 * DAY, "critical",
                   pace("running_out", 55.0, 151.0, now + DAY + 9 * HOUR), now),
            window("model:spark", "Spark", 100.0, 47 * MINUTE, 5 * HOUR, "critical", pace("spent", 84.0), now),
        ],
        now,
        owner="headroom",
        usage_home="~/.local/share/headroom/homes/codex-9f8e7d6c5b4a",
        notices=[{"tone": "warning", "text": "Weekly limit shared with Codex Cloud"}],
    )


def claude_personal(now):
    return account(
        "claude:0a1b2c3d4e5f", "claude", "personal", "me@example.org", "Max 5x", "error",
        [
            window("session", "Session", 0.0, None, 5 * HOUR, "good", pace("untracked"), now),
            window("weekly", "Weekly", 64.0, 2 * DAY + 5 * HOUR, 7 * DAY, "good", pace("healthy", 70.0, 88.0), now),
            window("model:opus", "Opus", 12.0, 2 * DAY + 5 * HOUR, 7 * DAY, "good", pace("healthy", 70.0, 17.0), now),
        ],
        now,
        error={"kind": "invalid_response", "message": "invalid response: HTTP 503 from api.anthropic.com"},
        balances=[{"id": "extra_usage", "label": "Extra usage", "kind": "count", "value": 1200, "unit": "requests"}],
    )


def claude_team(now):
    return account(
        "claude:5e4d3c2b1a0f", "claude", "team", "dev@example.com", "Team", "signed_out", [], now,
        error={"kind": "sign_in_expired", "message": "sign-in expired, open the CLI to sign in again"},
        source="cache",
    )


def codex_lapsed(now):
    return account(
        "codex:7c6b5a4f3e2d", "codex", "old", "old@example.net", "Plus", "no_subscription", [], now,
        error={"kind": "no_subscription", "message": "ChatGPT Plus ended on Sep 20, usage limits need an active plan"},
        updated_at=None,
        source=None,
        usage_home="~/.local/share/headroom/homes/codex-7c6b5a4f3e2d",
        owner="headroom",
    )


CODEX_MIX = [("gpt-5.5", 62, 78), ("gpt-5.5-mini", 21, 12), ("gpt-5.4-codex", 11, 8), ("o4-mini", 6, 2)]
CLAUDE_MIX = [
    ("claude-opus-4-5", 34, 61),
    ("claude-sonnet-4-5", 38, 27),
    ("claude-haiku-4-5", 12, 4),
    ("claude-opus-4-1", 5, 5),
    ("claude-sonnet-4", 6, 2),
    ("claude-3-5-haiku", 5, 1),
]


def full_usage(now):
    return [
        usage_entry("codex", "~/.codex", now, 3,
                    (totals(4_812_000, 14_370_000, CODEX_MIX), totals(9_120_000, 21_880_000, CODEX_MIX),
                     totals(182_400_000, 463_120_000, CODEX_MIX)), 61_000),
        usage_entry("claude", "~/.claude", now, 5,
                    (totals(1_203_448, 4_050_000, CLAUDE_MIX), totals(2_400_000, 8_300_000, CLAUDE_MIX),
                     totals(35_812_904, 96_400_000, CLAUDE_MIX, (412_000, ["claude-next"]))), 23_000),
    ]


TONE_RANK = {"neutral": 0, "good": 1, "warning": 2, "critical": 3}
DEFAULT_SETTINGS = {
    "refresh_interval_secs": 300,
    "notifications": {"almost_out": True, "cutting_it_close": True, "will_run_out": True, "reset": False},
    "headline": {"mode": "auto"},
    "reduced_motion": False,
    "display": {
        "theme": "system",
        "language": "system",
        "value_mode": "left",
        "reset_format": "countdown",
        "panel_label": "percent",
        "show_spend": True,
        "show_account_spend": True,
        "show_trend": True,
        "show_forecast": True,
        "translucent": False,
        "hidden_windows": {},
    },
}


def headline_for(entry, window_entry):
    return {
        "account_id": entry["id"],
        "window": window_entry["id"],
        "provider": entry["provider"],
        "account_label": entry["label"] or entry["email"],
        "window_label": window_entry["label"],
        "used_percent": window_entry["used_percent"],
        "remaining_percent": window_entry["remaining_percent"],
        "tone": window_entry["tone"],
    }


def candidates(accounts):
    return [(entry, w) for entry in accounts if not entry["hidden"] for w in entry["windows"] if not w["hidden"]]


def choose_headline(accounts, *targets):
    pool = candidates(accounts)
    for target in targets:
        match = next((pair for pair in pool if (pair[0]["id"], pair[1]["id"]) == target), None)
        if match:
            return headline_for(*match)
    if not pool:
        return None
    return headline_for(*max(pool, key=lambda pair: (TONE_RANK[pair[1]["tone"]], -pair[1]["remaining_percent"])))


def assemble(now, accounts, usage, preferred, **extra):
    state = {
        "version": 1,
        "generated_at": iso(now),
        "next_refresh_at": iso(now + 3 * MINUTE + timedelta(seconds=10)),
        "last_success_at": iso(now - 2 * MINUTE),
        "offline": False,
        "headline": choose_headline(accounts, preferred),
        "display": DEFAULT_SETTINGS["display"],
        "accounts": accounts,
        "usage": [published_usage(entry) for entry in usage],
        "spend": {period: published_spend(period_spend(usage, period)) for period in PERIODS},
    }
    state.update(extra)
    return state


WORK_SESSION = ("codex:1a2b3c4d5e6f", "session")


def full_accounts(now):
    return [codex_work(now), codex_personal(now), claude_personal(now), claude_team(now)]


def full_state(now):
    return assemble(now, full_accounts(now), full_usage(now), WORK_SESSION)


def critical_state(now):
    return assemble(now, full_accounts(now), full_usage(now), ("codex:9f8e7d6c5b4a", "weekly"))


def offline_state(now):
    accounts = full_accounts(now)
    for entry in accounts:
        entry["status"] = "error"
        entry["error"] = {"kind": "network", "message": "network error: could not resolve host"}
    return assemble(now, accounts, full_usage(now), WORK_SESSION,
                    offline=True, next_refresh_at=iso(now + 4 * MINUTE), last_success_at=iso(now - 47 * MINUTE))


def no_subscription_state(now):
    accounts = [codex_work(now), codex_lapsed(now), claude_personal(now)]
    return assemble(now, accounts, full_usage(now), WORK_SESSION)


def single_state(now):
    return assemble(now, [codex_work(now)], full_usage(now)[:1], WORK_SESSION)


def empty_state(now):
    return assemble(now, [], [], None, next_refresh_at=None, last_success_at=None)


SCENARIOS = {
    "full": full_state,
    "empty": empty_state,
    "offline": offline_state,
    "single": single_state,
    "critical": critical_state,
    "no_subscription": no_subscription_state,
}


def build(scenario, now=None):
    return SCENARIOS[scenario](now or datetime.now(timezone.utc))
