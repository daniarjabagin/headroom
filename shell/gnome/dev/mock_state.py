from datetime import datetime, timedelta, timezone

MINUTE = timedelta(minutes=1)
HOUR = timedelta(hours=1)
DAY = timedelta(days=1)
PERIODS = ("today", "yesterday", "last_30_days")
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


def totals(total_tokens, cost_micros, unpriced=None):
    unpriced_tokens, unpriced_models = unpriced or (0, [])
    return {
        "tokens": tokens(total_tokens),
        "cost_usd_micros": cost_micros,
        "partial": unpriced_tokens > 0,
        "unpriced_tokens": unpriced_tokens,
        "unpriced_models": unpriced_models,
    }


def daily(now, seed, scale):
    days = []
    for offset in range(29, -1, -1):
        wave = (seed * (offset + 3) * 7919) % 100
        value = 0 if wave < 18 else wave * scale
        date = (now - offset * DAY).strftime("%Y-%m-%d")
        days.append({"date": date, "total_tokens": value, "cost_usd_micros": value * 3, "partial": False})
    return days


def usage_entry(provider, home, now, seed, periods, models, scale):
    entry = {"provider": provider, "usage_home": home}
    entry.update(zip(PERIODS, periods))
    entry["daily"] = daily(now, seed, scale)
    entry["models"] = [
        {"model": name, "total_tokens": total, "cost_usd_micros": cost, "partial": cost == 0}
        for name, total, cost in models
    ]
    return entry


def provider_spend(usage, provider, period):
    entries = [entry[period] for entry in usage if entry["provider"] == provider]
    return {
        "provider": provider,
        "cost_usd_micros": sum(t["cost_usd_micros"] for t in entries),
        "total_tokens": sum(t["tokens"]["total"] for t in entries),
        "partial": any(t["partial"] for t in entries),
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
            window("model:spark", "Spark", 100.0, 4 * HOUR, 5 * HOUR, "critical", pace("spent", 20.0), now),
        ],
        now,
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


def full_usage(now):
    return [
        usage_entry("codex", "~/.codex", now, 3,
                    (totals(4_812_000, 14_370_000), totals(9_120_000, 21_880_000),
                     totals(182_400_000, 463_120_000)),
                    [("gpt-5.5", 150_000_000, 401_000_000), ("gpt-5.5-mini", 32_400_000, 62_120_000)], 61_000),
        usage_entry("claude", "~/.claude", now, 5,
                    (totals(1_203_448, 4_050_000), totals(2_400_000, 8_300_000),
                     totals(35_812_904, 96_400_000, (412_000, ["claude-next"]))),
                    [("claude-opus", 30_000_000, 90_000_000), ("claude-next", 412_000, 0)], 23_000),
    ]


def assemble(now, accounts, usage, headline, **extra):
    state = {
        "version": 1,
        "generated_at": iso(now),
        "next_refresh_at": iso(now + 3 * MINUTE + timedelta(seconds=10)),
        "last_success_at": iso(now - 2 * MINUTE),
        "offline": False,
        "headline": headline,
        "accounts": accounts,
        "usage": usage,
        "spend": {period: period_spend(usage, period) for period in PERIODS},
    }
    state.update(extra)
    return state


def headline(account_id, window_id, remaining, tone):
    return {"account_id": account_id, "window": window_id, "remaining_percent": remaining, "tone": tone}


def full_accounts(now):
    return [codex_work(now), codex_personal(now), claude_personal(now), claude_team(now)]


def full_state(now):
    return assemble(now, full_accounts(now), full_usage(now), headline("codex:1a2b3c4d5e6f", "session", 62.0, "good"))


def critical_state(now):
    return assemble(now, full_accounts(now), full_usage(now), headline("codex:9f8e7d6c5b4a", "weekly", 17.0, "critical"))


def offline_state(now):
    accounts = full_accounts(now)
    for entry in accounts:
        entry["status"] = "error"
        entry["error"] = {"kind": "network", "message": "network error: could not resolve host"}
    return assemble(now, accounts, full_usage(now), headline("codex:1a2b3c4d5e6f", "session", 62.0, "good"),
                    offline=True, next_refresh_at=iso(now + 4 * MINUTE), last_success_at=iso(now - 47 * MINUTE))


def single_state(now):
    return assemble(now, [codex_work(now)], full_usage(now)[:1], headline("codex:1a2b3c4d5e6f", "session", 62.0, "good"))


def empty_state(now):
    return assemble(now, [], [], None, next_refresh_at=None, last_success_at=None)


SCENARIOS = {
    "full": full_state,
    "empty": empty_state,
    "offline": offline_state,
    "single": single_state,
    "critical": critical_state,
}


def build(scenario, now=None):
    return SCENARIOS[scenario](now or datetime.now(timezone.utc))
