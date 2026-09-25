from datetime import timedelta

from mock_common import DAY, HOUR, MINUTE, iso
from mock_registry import PROVIDER_NAMES

TRACKED = ("healthy", "close")
SIGN_IN_ERRORS = ("not_signed_in", "sign_in_expired", "api_key_only")
WAITING_ERRORS = ("no_subscription", "rate_limited", "unsupported", "no_provider")
CLI_LOGINS = {"claude": "claude auth login", "codex": "codex login"}
IDLE_SECS = 300
LIVE_SECS = 60
ACCOUNT_KEYS = ("id", "provider", "provider_name", "label", "email", "plan", "owner", "hidden", "status", "error",
                "recovery", "updated_at", "source", "windows", "balances", "notices", "usage_home", "refresh",
                "collapsed")


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


def refresh(now, seconds, mode="idle", reason=None):
    return {
        "mode": mode,
        "interval_secs": LIVE_SECS if mode == "live" else IDLE_SECS,
        "next_at": iso(now + timedelta(seconds=seconds)),
        "reason": reason or ("activity" if mode == "live" else "schedule"),
    }


def recovery(entry):
    error = entry.get("error")
    if not error or error["kind"] in WAITING_ERRORS:
        return None
    if error["kind"] not in SIGN_IN_ERRORS:
        return {"action": "retry"}
    if entry["owner"] == "headroom":
        return {"action": "sign_in", "account_id": entry["id"]}
    command = CLI_LOGINS.get(entry["provider"])
    return {"action": "cli_login", "command": command} if command else {"action": "retry"}


def default_home(account_id, provider):
    if provider in ("codex", "claude"):
        return f"~/.{provider}"
    return f"~/.local/share/headroom/accounts/{provider}/{account_id.split(':')[1]}"


def account(account_id, provider, label, email, plan, status, windows, now, **extra):
    base = {
        "id": account_id,
        "provider": provider,
        "provider_name": PROVIDER_NAMES[provider],
        "label": label,
        "email": email,
        "plan": plan,
        "hidden": False,
        "owner": "cli",
        "status": status,
        "error": None,
        "updated_at": iso(now - (3 * HOUR if status == "stale" else 2 * MINUTE)),
        "source": "live",
        "windows": windows,
        "balances": [],
        "notices": [],
        "usage_home": default_home(account_id, provider),
        "refresh": refresh(now, 190),
        "collapsed": False,
    }
    base.update(extra)
    base.setdefault("recovery", recovery(base))
    return {key: base[key] for key in ACCOUNT_KEYS}


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
        refresh=refresh(now, 270),
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
        refresh=refresh(now, 240, "live", "backoff"),
    )


def claude_team(now):
    return account(
        "claude:5e4d3c2b1a0f", "claude", "team", "dev@example.com", "Team", "signed_out", [], now,
        error={"kind": "sign_in_expired", "message": "sign-in expired, open the CLI to sign in again"},
        source="cache",
        recovery={"action": "retry"},
        refresh=refresh(now, 480, "live", "backoff"),
    )


def codex_lapsed(now):
    return account(
        "codex:7c6b5a4f3e2d", "codex", "old", "old@example.net", "Plus", "no_subscription", [], now,
        error={"kind": "no_subscription", "message": "ChatGPT Plus ended on Sep 20, usage limits need an active plan"},
        updated_at=None,
        source=None,
        usage_home="~/.local/share/headroom/homes/codex-7c6b5a4f3e2d",
        owner="headroom",
        refresh=refresh(now, 3600, reason="hold"),
    )


def openrouter_key(now):
    return account(
        "openrouter:3c2b1a0f9e8d", "openrouter", None, None, None, "fresh", [], now,
        owner="headroom",
        balances=[{"id": "credits", "label": "Credits", "kind": "usd", "usd_micros": 7_420_000}],
        refresh=refresh(now, 250),
    )


def grok_weekly(now):
    return account(
        "grok:6d5c4b3a2f1e", "grok", None, "dev@example.com", "SuperGrok", "fresh",
        [window("weekly", "Weekly", 42.0, 3 * DAY + 4 * HOUR, 7 * DAY, "good", pace("healthy", 55.0, 76.0), now)],
        now,
        owner="headroom",
        notices=[{"tone": "good", "text": "Extra usage on, cap 50"}],
        refresh=refresh(now, 290),
    )


def full_accounts(now):
    return [codex_work(now), codex_personal(now), claude_personal(now), claude_team(now), openrouter_key(now),
            grok_weekly(now)]


def showcase_accounts(now):
    claude = account(
        "claude:0a1b2c3d4e5f", "claude", "personal", "me@example.org", "Max 5x", "fresh",
        [
            window("session", "Session", 24.0, 3 * HOUR + 12 * MINUTE, 5 * HOUR, "good",
                   pace("healthy", 36.0, 67.0), now),
            window("weekly", "Weekly", 68.0, 2 * DAY + 14 * HOUR, 7 * DAY, "warning",
                   pace("running_out", 63.4, 107.0, now + 2 * DAY + 2 * HOUR), now),
            window("model:opus", "Opus", 18.0, 2 * DAY + 14 * HOUR, 7 * DAY, "good",
                   pace("healthy", 63.4, 28.0), now),
        ],
        now,
    )
    codex = account(
        "codex:1a2b3c4d5e6f", "codex", "work", "dev@example.com", "Pro", "fresh",
        [
            window("session", "Session", 31.0, 2 * HOUR + 41 * MINUTE, 5 * HOUR, "good",
                   pace("healthy", 46.3, 67.0), now),
            window("weekly", "Weekly", 27.0, 4 * DAY + 6 * HOUR, 7 * DAY, "good", pace("healthy", 39.3, 69.0), now),
        ],
        now,
    )
    copilot = account(
        "copilot:4b3a2f1e0d9c", "copilot", None, "dev@example.com", "Individual Pro", "fresh",
        [window("credits", "Credits", 44.0, 11 * DAY, 30 * DAY, "good", pace("healthy", 63.3, 70.0), now)],
        now,
        provider_name="Copilot",
    )
    grok = account(
        "grok:6d5c4b3a2f1e", "grok", None, "dev@example.com", "SuperGrok", "fresh",
        [window("weekly", "Weekly", 36.0, 3 * DAY + 4 * HOUR, 7 * DAY, "good", pace("healthy", 54.8, 66.0), now)],
        now,
    )
    return [claude, codex, copilot, grok]
