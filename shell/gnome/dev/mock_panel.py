from mock_common import TONE_RANK, worst_tone
from mock_registry import PROVIDER_ORDER

QUIET_STATUSES = ("no_subscription",)
ATTENTION_STATUSES = ("signed_out", "error", "no_subscription")
LOUD_TONES = ("warning", "critical")
AUTO_LIMITS = 2
MAX_LIMITS = 3


def account_item(entry, window):
    return {
        "account_id": entry["id"],
        "window": window["id"],
        "provider": entry["provider"],
        "provider_name": entry["provider_name"],
        "account_label": entry["label"] or entry["email"] or entry["provider_name"],
        "window_label": window["label"],
        "used_percent": window["used_percent"],
        "remaining_percent": window["remaining_percent"],
        "tone": window["tone"],
        "combined": False,
        "account_count": 1,
        "even_pace_percent": window["pace"]["even_pace_percent"],
        "members": [entry["id"]],
    }


def group_item(group, window):
    share = window["capacity_percent"] / 100
    even = window["pace"]["even_pace_percent"]
    return {
        "account_id": window["segments"][0]["account_id"],
        "window": window["id"],
        "provider": group["provider"],
        "provider_name": group["provider_name"],
        "account_label": None,
        "window_label": window["label"],
        "used_percent": window["used_percent"] / share,
        "remaining_percent": window["remaining_percent"] / share,
        "tone": window["tone"],
        "combined": True,
        "account_count": len(window["segments"]),
        "even_pace_percent": None if even is None else even / share,
        "members": group["account_ids"],
    }


def account_candidates(entry):
    if entry["hidden"] or entry["status"] in QUIET_STATUSES:
        return []
    return [account_item(entry, window) for window in entry["windows"] if not window["hidden"]]


def candidates(accounts, groups):
    found = []
    for entry in accounts:
        group = next((g for g in groups if entry["id"] in g["account_ids"]), None)
        if group is None:
            found += account_candidates(entry)
        elif group["account_ids"][0] == entry["id"]:
            found += [group_item(group, window) for window in group["windows"] if window["segments"]]
    return found


def headline_of(item):
    return {key: value for key, value in item.items() if key not in ("even_pace_percent", "members")}


def matching(pool, target):
    return next((item for item in pool if target and target[0] in item["members"] and item["window"] == target[1]),
                None)


def most_critical(pool):
    return sorted(pool, key=lambda item: (-TONE_RANK[item["tone"]], item["remaining_percent"]))


def choose_headline(pool, *targets):
    for target in targets:
        match = matching(pool, target)
        if match:
            return match
    ranked = most_critical(pool)
    return ranked[0] if ranked else None


def panel_item(item, value_mode):
    value = item["used_percent"] if value_mode == "used" else item["remaining_percent"]
    return {**headline_of(item), "value_percent": value, "even_pace_percent": item["even_pace_percent"],
            "logo": item["provider"]}


def limit_items(pool, limits):
    chosen = []
    for limit in limits:
        match = matching(pool, (limit["account_id"], limit["window"]))
        if match and match not in chosen:
            chosen.append(match)
    return chosen[:MAX_LIMITS]


def panel_items(display, pool, headline):
    if display["panel_mode"] == "icon":
        return []
    if display["panel_mode"] == "headline":
        chosen = [headline] if headline else []
    elif display["panel_limits"]:
        chosen = limit_items(pool, display["panel_limits"])
    else:
        chosen = most_critical(pool)[:AUTO_LIMITS]
    return [panel_item(item, display["value_mode"]) for item in chosen]


def needs_attention(entry):
    return entry["error"] is not None or entry["status"] in ATTENTION_STATUSES


def loud(windows):
    return any(window["tone"] in LOUD_TONES for window in windows if not window.get("hidden"))


def collapsed_account(display, entry):
    return (display["collapse_unstarred"] and entry["id"] not in display["starred_accounts"]
            and not needs_attention(entry) and not loud(entry["windows"]))


def collapsed_group(display, group, accounts):
    members = [entry for entry in accounts if entry["id"] in group["account_ids"]]
    return (display["collapse_unstarred"] and not set(group["account_ids"]) & set(display["starred_accounts"])
            and not any(needs_attention(entry) for entry in members) and not loud(group["windows"]))


def adjusted_refresh(entry, settings):
    current = entry["refresh"]
    live = current["mode"] == "live" and settings["adaptive_refresh"]
    reason = current["reason"] if current["reason"] in ("hold", "backoff") else ("activity" if live else "schedule")
    return {
        "mode": "live" if live else "idle",
        "interval_secs": 60 if live else settings["refresh_interval_secs"],
        "next_at": None if entry["status"] == "refreshing" else current["next_at"],
        "reason": reason,
    }


def shown_statuses(statuses, accounts, settings):
    if not settings["status_pages"]["enabled"]:
        return []
    providers = {entry["provider"] for entry in accounts if not entry["hidden"]}
    listed = [entry for entry in statuses if entry["provider"] in providers]
    return sorted(listed, key=lambda entry: PROVIDER_ORDER.index(entry["provider"]))


def finish(state, settings, groups, pinned=None):
    display = settings["display"]
    pool = candidates(state["accounts"], groups)
    headline = choose_headline(pool, pinned, state.pop("preferred", None))
    for entry in state["accounts"]:
        entry["refresh"] = adjusted_refresh(entry, settings)
        entry["collapsed"] = collapsed_account(display, entry)
    for group in groups:
        group["collapsed"] = collapsed_group(display, group, state["accounts"])
    state.update({
        "display": display,
        "headline": headline_of(headline) if headline else None,
        "panel_items": panel_items(display, pool, headline),
        "panel_tone": worst_tone(item["tone"] for item in pool),
        "combined": groups,
        "provider_status": shown_statuses(state["provider_status"], state["accounts"], settings),
    })
    return state
