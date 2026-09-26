import copy
import re

DEFAULT_DISPLAY = {
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
    "combine_accounts": False,
    "hidden_windows": {},
    "density": "normal",
    "time_format": "auto",
    "panel_mode": "headline",
    "panel_indicator": "ring",
    "panel_limits": [],
    "panel_position": {"box": "right", "index": 0},
    "spend_period": "30d",
    "spend_unit": "cost",
    "spend_breakdown": "models",
    "show_breakdown": True,
    "starred_accounts": [],
    "collapse_unstarred": False,
    "hide_on_screen_share": True,
}
DEFAULT_SETTINGS = {
    "refresh_interval_secs": 300,
    "adaptive_refresh": True,
    "notifications": {
        "almost_out": True,
        "cutting_it_close": True,
        "will_run_out": True,
        "reset": False,
        "threshold_percent": 10,
        "provider_thresholds": {},
        "quiet_hours": {"enabled": False, "from": "22:00", "to": "08:00", "allow_critical": True},
    },
    "headline": {"mode": "auto"},
    "reduced_motion": False,
    "display": DEFAULT_DISPLAY,
    "updates": {"check": True},
    "status_pages": {"enabled": False},
    "shortcuts": {"open": ""},
    "logging": {"level": "info"},
    "onboarding": {"completed": True},
}
CHOICES = {
    ("display", "theme"): ("system", "light", "dark"),
    ("display", "language"): ("system", "en", "ru"),
    ("display", "value_mode"): ("left", "used"),
    ("display", "reset_format"): ("countdown", "exact"),
    ("display", "panel_label"): ("percent", "window", "none"),
    ("display", "density"): ("normal", "compact"),
    ("display", "time_format"): ("auto", "12h", "24h"),
    ("display", "panel_mode"): ("headline", "several", "icon"),
    ("display", "panel_indicator"): ("ring", "bar", "none"),
    ("display", "spend_period"): ("today", "yesterday", "7d", "30d"),
    ("display", "spend_unit"): ("cost", "tokens", "cost_per_mtok"),
    ("display", "spend_breakdown"): ("models", "projects"),
    ("logging", "level"): ("error", "warn", "info", "debug"),
}
CLOCK = re.compile(r"^([01]\d|2[0-3]):[0-5]\d$")
ACCELERATOR = re.compile(r"^(<[A-Za-z]+>)*[A-Za-z0-9_]+$")
FREE_FORM = ("hidden_windows", "provider_thresholds")


def merged(defaults, raw):
    if not isinstance(defaults, dict) or not isinstance(raw, dict):
        return copy.deepcopy(raw if raw is not None and type(raw) is type(defaults) else defaults)
    return {key: copy.deepcopy(raw[key]) if key in FREE_FORM and isinstance(raw.get(key), dict)
            else merged(value, raw.get(key))
            for key, value in defaults.items()}


def merge_patch(target, patch):
    if not isinstance(patch, dict):
        return copy.deepcopy(patch)
    result = copy.deepcopy(target) if isinstance(target, dict) else {}
    for key, value in patch.items():
        if value is None:
            result.pop(key, None)
        else:
            result[key] = merge_patch(result.get(key), value)
    return result


def check_choices(settings):
    for (section, key), allowed in CHOICES.items():
        if settings[section][key] not in allowed:
            raise ValueError(f"unknown {section}.{key} value {settings[section][key]!r}")


def check_limits(display):
    limits = display["panel_limits"]
    if any(not isinstance(limit, dict) or set(limit) != {"account_id", "window"} or not all(limit.values())
           for limit in limits):
        raise ValueError("display.panel_limits entries need a non-empty account_id and window")
    distinct = [limit for index, limit in enumerate(limits) if limit not in limits[:index]]
    if len(distinct) > 3:
        raise ValueError("display.panel_limits holds at most 3 limits")
    display["panel_limits"] = distinct
    position = display["panel_position"]
    if position["box"] not in ("left", "center", "right") or not isinstance(position["index"], int) \
            or position["index"] < 0:
        raise ValueError("display.panel_position needs a box and an index >= 0")
    display["starred_accounts"] = list(dict.fromkeys(entry for entry in display["starred_accounts"] if entry))


def check_notifications(notifications):
    if not 1 <= notifications["threshold_percent"] <= 50:
        raise ValueError("notifications.threshold_percent must be 1-50")
    if any(not isinstance(value, int) or not 0 <= value <= 50
           for value in notifications["provider_thresholds"].values()):
        raise ValueError("notifications.provider_thresholds values must be 0-50")
    quiet = notifications["quiet_hours"]
    if not CLOCK.match(quiet["from"]) or not CLOCK.match(quiet["to"]):
        raise ValueError("notifications.quiet_hours times must be HH:MM")
    if quiet["enabled"] and quiet["from"] == quiet["to"]:
        raise ValueError("notifications.quiet_hours from and to must differ")


def check_misc(settings):
    if not 60 <= settings["refresh_interval_secs"] <= 3600:
        raise ValueError("refresh_interval_secs must be 60-3600")
    shortcut = settings["shortcuts"]["open"]
    if shortcut and (len(shortcut) > 64 or not ACCELERATOR.match(shortcut)):
        raise ValueError(f"shortcuts.open is not a GTK accelerator: {shortcut!r}")


def pinned_headline(raw):
    headline = raw.get("headline") if isinstance(raw.get("headline"), dict) else {}
    if headline.get("mode") == "pinned" and headline.get("account_id") and headline.get("window"):
        return {"mode": "pinned", "account_id": headline["account_id"], "window": headline["window"]}
    return {"mode": "auto"}


def normalized_settings(raw):
    settings = merged(DEFAULT_SETTINGS, raw)
    settings["headline"] = pinned_headline(raw)
    check_choices(settings)
    check_limits(settings["display"])
    check_notifications(settings["notifications"])
    check_misc(settings)
    return settings


def reset_settings(current):
    return {**copy.deepcopy(DEFAULT_SETTINGS), "onboarding": copy.deepcopy(current["onboarding"])}


def scenario_settings(patch):
    return normalized_settings(merge_patch(DEFAULT_SETTINGS, patch or {}))
