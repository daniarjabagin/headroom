from datetime import timedelta

from mock_accounts import (claude_personal, codex_lapsed, codex_work, full_accounts, showcase_accounts)
from mock_common import MINUTE, iso
from mock_spend import published_usage, spend, totals, usage_entry

CODEX_MIX = [("gpt-5.5", 62, 78), ("gpt-5.5-mini", 21, 12), ("gpt-5.4-codex", 11, 8), ("o4-mini", 6, 2)]
CLAUDE_MIX = [
    ("claude-opus-4-5", 34, 61),
    ("claude-sonnet-4-5", 38, 27),
    ("claude-haiku-4-5", 12, 4),
    ("claude-opus-4-1", 5, 5),
    ("claude-sonnet-4", 6, 2),
    ("claude-3-5-haiku", 5, 1),
]
OPENCODE_MIX = [("kimi-k2", 58, 41), ("glm-4.6", 42, 59)]
GROK_MIX = [("grok-4", 70, 82), ("grok-code-fast-1", 30, 18)]
WORK_SESSION = ("codex:1a2b3c4d5e6f", "session")
CLAUDE_WEEKLY = ("claude:0a1b2c3d4e5f", "weekly")
SAMPLE_CHECKED_AT = "2026-09-23T04:00:00Z"


def full_usage(now):
    return [
        usage_entry("codex", "~/.codex", now, 3,
                    (totals(4_812_000, 14_370_000, CODEX_MIX), totals(9_120_000, 21_880_000, CODEX_MIX),
                     totals(182_400_000, 463_120_000, CODEX_MIX)), 61_000),
        usage_entry("claude", "~/.claude", now, 5,
                    (totals(1_203_448, 4_050_000, CLAUDE_MIX), totals(2_400_000, 8_300_000, CLAUDE_MIX),
                     totals(35_812_904, 96_400_000, CLAUDE_MIX, (412_000, ["claude-next"]))), 23_000),
        usage_entry("opencode", "~/.local/share/opencode", now, 7,
                    (totals(640_000, 1_310_000, OPENCODE_MIX), totals(0, 0, OPENCODE_MIX),
                     totals(12_480_000, 24_700_000, OPENCODE_MIX)), 9_000),
    ]


def showcase_usage(now):
    return [
        usage_entry("claude", "~/.claude", now, 5,
                    (totals(1_904_220, 11_240_000, CLAUDE_MIX), totals(3_112_000, 17_860_000, CLAUDE_MIX),
                     totals(48_310_500, 262_400_000, CLAUDE_MIX)), 31_000),
        usage_entry("codex", "~/.codex", now, 3,
                    (totals(3_210_400, 8_420_000, CODEX_MIX), totals(5_004_000, 12_930_000, CODEX_MIX),
                     totals(96_120_000, 231_750_000, CODEX_MIX)), 44_000),
        usage_entry("grok", "~/.grok", now, 11,
                    (totals(812_000, 2_350_000, GROK_MIX), totals(1_020_000, 2_910_000, GROK_MIX),
                     totals(14_300_000, 38_600_000, GROK_MIX)), 8_000),
    ]


def incident(provider, indicator, tone, title, stage, started_at, url):
    return {"provider": provider, "indicator": indicator, "tone": tone, "title": title, "stage": stage,
            "started_at": iso(started_at), "url": url}


def all_clear(provider, url):
    return {"provider": provider, "indicator": "none", "tone": "neutral", "title": None, "stage": None,
            "started_at": None, "url": url}


def minor_incident(now):
    return [
        all_clear("codex", "https://status.openai.com"),
        incident("claude", "minor", "warning", "Elevated errors on Claude Code", "identified", now - 48 * MINUTE,
                 "https://stspg.io/abc123"),
    ]


def assemble(now, accounts, usage, preferred, **extra):
    state = {
        "version": 1,
        "app_version": "0.5.1",
        "generated_at": iso(now),
        "next_refresh_at": iso(now + 3 * MINUTE + timedelta(seconds=10)),
        "last_success_at": iso(now - 2 * MINUTE),
        "offline": False,
        "headline": None,
        "display": None,
        "accounts": accounts,
        "combined": [],
        "provider_status": minor_incident(now),
        "usage": [published_usage(entry) for entry in usage],
        "spend": spend(usage),
        "update": None,
        "update_check": {"checked_at": SAMPLE_CHECKED_AT},
        "panel_items": [],
        "panel_tone": None,
        "preferred": preferred,
        "usage_source": usage,
    }
    state.update(extra)
    return state


def full_state(now):
    return assemble(now, full_accounts(now), full_usage(now), WORK_SESSION)


def showcase_state(now):
    return assemble(now, showcase_accounts(now), showcase_usage(now), CLAUDE_WEEKLY)


UPDATE_COMMANDS = {
    "self": "headroom update",
    "package": "Download the new Arch package from {url} and install it with sudo pacman -U",
    "unknown": "{url}",
}


def available_update(install, now):
    url = "https://github.com/daniarjabagin/headroom/releases/tag/v0.5.0"
    return {
        "version": "0.5.0",
        "url": url,
        "published_at": iso(now - timedelta(days=2)),
        "install": install,
        "command": UPDATE_COMMANDS[install].format(url=url),
    }


def showcase_update_state(install):
    return lambda now: {**showcase_state(now), "update": available_update(install, now)}


def critical_state(now):
    return assemble(now, full_accounts(now), full_usage(now), ("codex:9f8e7d6c5b4a", "weekly"))


def offline_state(now):
    accounts = full_accounts(now)
    for entry in accounts:
        entry["status"] = "error"
        entry["error"] = {"kind": "network", "message": "network error: could not resolve host"}
        entry["recovery"] = {"action": "retry"}
    return assemble(now, accounts, full_usage(now), WORK_SESSION, provider_status=[],
                    offline=True, next_refresh_at=iso(now + 4 * MINUTE), last_success_at=iso(now - 47 * MINUTE))


def no_subscription_state(now):
    accounts = [codex_work(now), codex_lapsed(now), claude_personal(now)]
    return assemble(now, accounts, full_usage(now), WORK_SESSION)


def single_state(now):
    return assemble(now, [codex_work(now)], full_usage(now)[:1], WORK_SESSION)


def empty_state(now):
    return assemble(now, [], [], None, next_refresh_at=None, last_success_at=None, provider_status=[])


SCENARIOS = {
    "full": full_state,
    "showcase": showcase_state,
    "showcase-update": showcase_update_state("self"),
    "showcase-update-package": showcase_update_state("package"),
    "showcase-update-unknown": showcase_update_state("unknown"),
    "empty": empty_state,
    "offline": offline_state,
    "single": single_state,
    "critical": critical_state,
    "no_subscription": no_subscription_state,
}
