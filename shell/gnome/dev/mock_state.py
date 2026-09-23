from datetime import datetime, timedelta, timezone

HOUR = timedelta(hours=1)
DAY = timedelta(days=1)


def iso(moment):
    return moment.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ") if moment else None


def pace(severity, even=None, projected=None, runs_out_at=None):
    return {
        "severity": severity,
        "even_pace_percent": even,
        "projected_percent": projected,
        "runs_out_at": iso(runs_out_at),
    }


def window(window_id, label, used, resets_in, period, tone, window_pace, now):
    remaining = None if used is None else max(0.0, 100.0 - used)
    return {
        "id": window_id,
        "label": label,
        "used_percent": used,
        "remaining_percent": remaining,
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


def totals(total_tokens, cost_micros, partial=False):
    return {"tokens": tokens(total_tokens), "cost_usd_micros": cost_micros, "partial": partial}


def daily(now, seed, scale):
    days = []
    for offset in range(29, -1, -1):
        wave = (seed * (offset + 3) * 7919) % 100
        value = 0 if wave < 18 else wave * scale
        date = (now - offset * DAY).strftime("%Y-%m-%d")
        days.append({"date": date, "total_tokens": value, "cost_usd_micros": value * 3})
    return days


def usage_entry(provider, home, now, seed, today, yesterday, month, scale):
    return {
        "provider": provider,
        "usage_home": home,
        "today": totals(*today),
        "yesterday": totals(*yesterday),
        "last_30_days": totals(*month),
        "daily": daily(now, seed, scale),
        "models": [],
    }


def account(account_id, provider, label, email, plan, status, windows, now, **extra):
    base = {
        "id": account_id,
        "provider": provider,
        "label": label,
        "email": email,
        "plan": plan,
        "status": status,
        "error": None,
        "updated_at": iso(now - (3 * HOUR if status == "stale" else timedelta(minutes=2))),
        "source": "live",
        "windows": windows,
        "balances": [],
        "notices": [],
        "usage_home": "~/.codex" if provider == "codex" else "~/.claude",
        "hidden": False,
    }
    base.update(extra)
    return base


def codex_work(now):
    return account(
        "codex:1a2b3c4d5e6f", "codex", "work", "dev@example.com", "Pro", "fresh",
        [
            window("session", "Session", 38.0, 2 * HOUR + timedelta(minutes=41), 5 * HOUR, "good",
                   pace("healthy", 46.0, 82.0), now),
            window("weekly", "Weekly", 19.0, 4 * DAY + 6 * HOUR, 7 * DAY, "good", pace("healthy", 38.0, 50.0), now),
        ],
        now,
        balances=[{"id": "credits", "label": "Credits", "usd_micros": 12500000}],
    )


def codex_personal(now):
    return account(
        "codex:9f8e7d6c5b4a", "codex", "personal", "me@example.org", "Plus", "stale",
        [
            window("session", "Session", 71.0, HOUR + timedelta(minutes=12), 5 * HOUR, "warning",
                   pace("close", 76.0, 96.0), now),
            window("weekly", "Weekly", 83.0, 3 * DAY + 2 * HOUR, 7 * DAY, "critical",
                   pace("running_out", 55.0, 151.0, now + DAY + 9 * HOUR), now),
            window("spark", "Spark", 100.0, 4 * HOUR, 5 * HOUR, "critical", pace("spent", 20.0), now),
        ],
        now,
    )


def claude_personal(now):
    return account(
        "claude:0a1b2c3d4e5f", "claude", "personal", "me@example.org", "Max 5x", "error",
        [
            window("session", "Session", 0.0, None, 5 * HOUR, "good", pace("untracked"), now),
            window("weekly", "Weekly", 64.0, 2 * DAY + 5 * HOUR, 7 * DAY, "good", pace("healthy", 70.0, 88.0), now),
            window("opus", "Opus", None, None, 7 * DAY, "neutral", pace("untracked"), now),
        ],
        now,
        error="HTTP 503 from api.anthropic.com",
    )


def claude_team(now):
    return account("claude:5e4d3c2b1a0f", "claude", "team", "dev@example.com", "Team", "signed_out", [], now)


def full_state(now):
    return {
        "version": 1,
        "generated_at": iso(now),
        "next_refresh_at": iso(now + timedelta(minutes=3, seconds=10)),
        "offline": False,
        "daemon_version": "0.1.0",
        "headline": {"account_id": "codex:1a2b3c4d5e6f", "window": "session", "remaining_percent": 62.0,
                     "tone": "good"},
        "accounts": [codex_work(now), codex_personal(now), claude_personal(now), claude_team(now)],
        "usage": [
            usage_entry("codex", "~/.codex", now, 3, (4_812_000, 14_370_000), (9_120_000, 21_880_000),
                        (182_400_000, 463_120_000), 61_000),
            usage_entry("claude", "~/.claude", now, 5, (1_203_448, 4_050_000), (2_400_000, 8_300_000),
                        (35_812_904, 96_400_000, True), 23_000),
        ],
    }


def empty_state(now):
    return {"version": 1, "generated_at": iso(now), "headline": None, "accounts": [], "usage": []}


def offline_state(now):
    state = full_state(now)
    state["offline"] = True
    state["last_success_at"] = iso(now - 47 * timedelta(minutes=1))
    for entry in state["accounts"]:
        if entry["status"] == "fresh":
            entry["status"] = "stale"
    return state


def single_state(now):
    state = full_state(now)
    state["accounts"] = [codex_work(now)]
    state["usage"] = state["usage"][:1]
    return state


def critical_state(now):
    state = full_state(now)
    state["headline"] = {"account_id": "codex:9f8e7d6c5b4a", "window": "weekly", "remaining_percent": 17.0,
                         "tone": "critical"}
    return state


SCENARIOS = {
    "full": full_state,
    "empty": empty_state,
    "offline": offline_state,
    "single": single_state,
    "critical": critical_state,
}


def build(scenario, now=None):
    return SCENARIOS[scenario](now or datetime.now(timezone.utc))
