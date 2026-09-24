from datetime import datetime, timezone

from mock_state import (DAY, HOUR, MINUTE, SCENARIOS as BASE_SCENARIOS, TRACKED, account, assemble, pace,
                        showcase_accounts, showcase_usage, window)

SEVERITY_TONES = {"healthy": "good", "close": "warning", "running_out": "critical", "spent": "critical"}
TONE_RANK = {"neutral": 0, "good": 1, "warning": 2, "critical": 3}
QUIET = ("signed_out", "no_subscription")
CLOSE_SHARE = 0.9


def combinable(entry):
    return not entry["hidden"] and entry["status"] not in QUIET and any(not w["hidden"] for w in entry["windows"])


def shown_window(entry, window_id):
    return next((w for w in entry["windows"] if w["id"] == window_id and not w["hidden"]), None)


def combined_severity(parts, capacity, projected):
    if any(w["pace"]["severity"] == "spent" for _, w in parts):
        return "spent"
    if projected is None:
        return "untracked"
    if projected > capacity:
        return "running_out"
    return "close" if projected > capacity * CLOSE_SHARE else "healthy"


def combined_pace(parts, capacity):
    projections = [w["pace"]["projected_percent"] for _, w in parts]
    evens = [w["pace"]["even_pace_percent"] for _, w in parts]
    projected = None if None in projections else sum(projections)
    severity = combined_severity(parts, capacity, projected)
    single_run_out = parts[0][1]["pace"]["runs_out_at"] if len(parts) == 1 else None
    return {
        "severity": severity,
        "even_pace_percent": None if None in evens else sum(evens),
        "projected_percent": projected,
        "spare_percent": capacity - projected if severity in TRACKED else None,
        "runs_out_at": single_run_out,
    }


def segment(entry, part):
    return {
        "account_id": entry["id"],
        "label": entry["label"] or entry["email"],
        "remaining_percent": part["remaining_percent"],
        "used_percent": part["used_percent"],
        "resets_at": part["resets_at"],
        "tone": part["tone"],
    }


def combined_window(window_id, members):
    parts = [(entry, shown_window(entry, window_id)) for entry in members if shown_window(entry, window_id)]
    capacity = 100 * len(parts)
    remaining = sum(w["remaining_percent"] for _, w in parts)
    window_pace = combined_pace(parts, capacity)
    resets = [w["resets_at"] for _, w in parts if w["resets_at"]]
    return {
        "id": window_id,
        "label": parts[0][1]["label"],
        "capacity_percent": capacity,
        "remaining_percent": remaining,
        "used_percent": capacity - remaining,
        "resets_at": min(resets) if resets else None,
        "tone": SEVERITY_TONES.get(window_pace["severity"], "neutral"),
        "pace": window_pace,
        "segments": [segment(entry, w) for entry, w in parts],
    }


def window_ids(members):
    ids = []
    for entry in members:
        ids += [w["id"] for w in entry["windows"] if not w["hidden"] and w["id"] not in ids]
    return ids


def group(members):
    return {
        "provider": members[0]["provider"],
        "provider_name": members[0]["provider_name"],
        "account_ids": [entry["id"] for entry in members],
        "accounts": [{"account_id": e["id"], "label": e["label"] or e["email"], "plan": e["plan"]} for e in members],
        "windows": [combined_window(window_id, members) for window_id in window_ids(members)],
    }


def combined_groups(accounts):
    by_provider = {}
    for entry in accounts:
        if combinable(entry):
            by_provider.setdefault(entry["provider"], []).append(entry)
    return [group(members) for members in by_provider.values() if len(members) > 1]


def combined_headline(headline, groups):
    if headline is None:
        return None
    for entry in groups:
        found = next((w for w in entry["windows"] if w["id"] == headline["window"]), None)
        if headline["account_id"] in entry["account_ids"] and found:
            share = found["capacity_percent"] / 100
            return {**headline, "account_id": None, "account_label": None, "combined": True,
                    "account_count": len(found["segments"]), "remaining_percent": found["remaining_percent"] / share,
                    "used_percent": found["used_percent"] / share, "tone": found["tone"]}
    return headline


def with_combined(state):
    groups = combined_groups(state["accounts"]) if state["display"].get("combine_accounts") else []
    return {**state, "combined": groups, "headline": combined_headline(state["headline"], groups)}


def codex_plus(now):
    return account(
        "codex:9f8e7d6c5b4a", "codex", "personal", "me@example.org", "Plus", "fresh",
        [
            window("session", "Session", 50.0, HOUR + 12 * MINUTE, 5 * HOUR, "warning",
                   pace("close", 76.0, 96.0), now),
            window("weekly", "Weekly", 71.0, 3 * DAY + 2 * HOUR, 7 * DAY, "critical",
                   pace("running_out", 55.0, 129.0, now + DAY + 9 * HOUR), now),
            window("model:spark", "Spark", 40.0, 2 * HOUR, 5 * HOUR, "good", pace("healthy", 60.0, 70.0), now),
        ],
        now,
        owner="headroom",
    )


def combined_state(now):
    claude, codex, copilot, grok = showcase_accounts(now)
    state = assemble(now, [claude, codex, codex_plus(now), copilot, grok], showcase_usage(now),
                     ("codex:1a2b3c4d5e6f", "weekly"))
    state["display"] = {**state["display"], "combine_accounts": True}
    return with_combined(state)


SCENARIOS = {**BASE_SCENARIOS, "combined": combined_state}


def build(scenario, now=None):
    return SCENARIOS[scenario](now or datetime.now(timezone.utc))

