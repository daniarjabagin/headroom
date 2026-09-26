from datetime import datetime, timezone

from mock_accounts import account, pace, refresh, showcase_accounts, window
from mock_combined import combined_groups, combined_state
from mock_common import DAY, HOUR, MINUTE
from mock_panel import finish
from mock_providers import providers_state
from mock_settings import scenario_settings
from mock_state import (CLAUDE_WEEKLY, SCENARIOS as BASE_SCENARIOS, assemble, full_state, incident, showcase_state,
                        showcase_usage)

SEVERAL_LIMITS = [
    {"account_id": "codex:1a2b3c4d5e6f", "window": "session"},
    {"account_id": "codex:9f8e7d6c5b4a", "window": "weekly"},
    {"account_id": "claude:0a1b2c3d4e5f", "window": "weekly"},
]
STATUS_ON = {"status_pages": {"enabled": True}}
SCENARIO_SETTINGS = {
    "full": STATUS_ON,
    "combined": {"display": {"combine_accounts": True}},
    "several": {"display": {"panel_mode": "several", "panel_indicator": "bar", "panel_limits": SEVERAL_LIMITS}},
    "several-auto": {"display": {"panel_mode": "several"}},
    "icon": {"display": {"panel_mode": "icon", "panel_label": "none"}},
    "collapsed": {"display": {"collapse_unstarred": True, "starred_accounts": ["claude:0a1b2c3d4e5f"]}},
    "incident": STATUS_ON,
    "onboarding": {"onboarding": {"completed": False}},
    "accounts": {"display": {"starred_accounts": ["codex:1a2b3c4d5e6f"]}},
}


def quiet_account(account_id, provider, email, plan, used, now):
    return account(
        account_id, provider, None, email, plan, "fresh",
        [window("weekly", "Weekly", used, 5 * DAY, 7 * DAY, "good", pace("healthy", 30.0, used * 2), now)],
        now,
        owner="headroom",
    )


def collapsed_state(now):
    extra = [quiet_account("cursor:1f2e3d4c5b6a", "cursor", "dev@example.com", "Pro", 12.0, now),
             quiet_account("warp:8e7d6c5b4a3f", "warp", "dev@example.com", "Build", 8.0, now)]
    return assemble(now, showcase_accounts(now) + extra, showcase_usage(now), CLAUDE_WEEKLY)


def live_state(now):
    state = showcase_state(now)
    for entry, seconds in zip(state["accounts"][:2], (35, 50)):
        entry["refresh"] = refresh(now, seconds, "live")
    return state


def incident_state(now):
    statuses = [
        incident("claude", "major", "critical", "Claude Code requests failing", "investigating", now - 17 * MINUTE,
                 "https://stspg.io/def456"),
        incident("codex", "maintenance", "warning", "Scheduled database maintenance", "in_progress",
                 now - HOUR, "https://status.openai.com/incidents/01K0MAINT"),
        incident("copilot", "minor", "warning", "Degraded Copilot completions", "monitoring", now - 2 * HOUR,
                 "https://www.githubstatus.com/incidents/k2m9"),
    ]
    return {**showcase_state(now), "provider_status": statuses}


def accounts_state(now):
    state = full_state(now)
    for entry in state["accounts"]:
        if entry["status"] == "signed_out":
            entry["owner"] = "headroom"
            entry["error"] = {"kind": "sign_in_expired", "message": "Sign-in expired"}
    return state


def spend_only_state(now):
    return assemble(now, [], showcase_usage(now), None)


SCENARIOS = {
    **BASE_SCENARIOS,
    "spend-only": spend_only_state,
    "combined": combined_state,
    "providers": providers_state,
    "several": full_state,
    "several-auto": full_state,
    "icon": full_state,
    "collapsed": collapsed_state,
    "live": live_state,
    "incident": incident_state,
    "onboarding": showcase_state,
    "accounts": accounts_state,
}


def initial_settings(scenario):
    return scenario_settings(SCENARIO_SETTINGS.get(scenario))


def base_state(scenario, now=None):
    return SCENARIOS[scenario](now or datetime.now(timezone.utc))


def finished(state, settings, pinned=None):
    state.pop("usage_source", None)
    combine = settings["display"]["combine_accounts"]
    groups = combined_groups(state["accounts"]) if combine else []
    return finish(state, settings, groups, pinned)


def build(scenario, now=None):
    return finished(base_state(scenario, now), initial_settings(scenario))
