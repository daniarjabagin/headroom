import json
import re
from datetime import date, timedelta

from mock_common import MINUTE, iso
from mock_registry import PROVIDER_NAMES, PROVIDER_ORDER
from mock_spend import cost_per_mtok, merge_models, scaled, shares_of, PROJECT_SHARES, tokens

QUERY_FIELDS = {"period", "since", "until", "by", "provider"}
GROUPINGS = ("model", "project", "provider", "day")
PERIOD_DAYS = {"today": (0, 0), "yesterday": (1, 1), "7d": (6, 0), "30d": (29, 0)}
RETENTION_DAYS = 33
ISO_DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
APP_VERSION = "0.6.0"


def parse_day(value, field):
    if not isinstance(value, str) or not ISO_DATE.match(value):
        raise ValueError(f"{field} must be YYYY-MM-DD")
    return date.fromisoformat(value)


def check_query(query):
    if not isinstance(query, dict):
        raise ValueError("spend query must be a JSON object")
    unknown = set(query) - QUERY_FIELDS
    if unknown:
        raise ValueError(f"unknown spend query field {sorted(unknown)[0]}")
    if query.get("by") not in GROUPINGS:
        raise ValueError("by must be model, project, provider or day")
    if ("period" in query) == ("since" in query):
        raise ValueError("give either period or since")
    if "provider" in query and query["provider"] not in PROVIDER_NAMES:
        raise ValueError(f"unknown provider {query['provider']}")


def resolve_range(query, today):
    if "period" in query:
        if query["period"] not in PERIOD_DAYS or "until" in query:
            raise ValueError("period must be today, yesterday, 7d or 30d, without until")
        first, last = PERIOD_DAYS[query["period"]]
        return today - timedelta(days=first), today - timedelta(days=last)
    since = parse_day(query["since"], "since")
    until = parse_day(query["until"], "until") if "until" in query else today
    if until > today or since > until:
        raise ValueError("since and until must not be after today, until not before since")
    if since < today - timedelta(days=RETENTION_DAYS):
        raise ValueError(f"since may be at most {RETENTION_DAYS} days ago")
    return since, until


def range_totals(entry, since, until, today):
    if since == until == today:
        return entry["today"]
    if since == until == today - timedelta(days=1):
        return entry["yesterday"]
    return scaled(entry["last_30_days"], (until - since).days + 1, 30)


def row(key, provider, total, cost, unpriced):
    return {"key": key, "provider": provider, "tokens": tokens(total), "cost_usd_micros": cost,
            "partial": unpriced > 0, "unpriced_tokens": unpriced,
            "cost_per_mtok_usd_micros": cost_per_mtok(cost, total - unpriced)}


def model_rows(per_provider):
    rows = []
    for provider, entries in per_provider.items():
        for model in merge_models(entry["models"] for entry in entries):
            unpriced = model["total_tokens"] if model["partial"] else 0
            rows.append(row(model["model"], provider, model["total_tokens"], model["cost_usd_micros"], unpriced))
    return rows


def provider_rows(per_provider):
    return [row(provider, provider, sum(e["tokens"]["total"] for e in entries),
                sum(e["cost_usd_micros"] for e in entries), sum(e["unpriced_tokens"] for e in entries))
            for provider, entries in per_provider.items()]


def project_rows(per_provider):
    providers = provider_rows(per_provider)
    columns = [(shares_of(p["tokens"]["total"]), shares_of(p["cost_usd_micros"])) for p in providers]
    return [row(project, None, sum(c[0][i] for c in columns), sum(c[1][i] for c in columns), 0)
            for i, (project, _) in enumerate(PROJECT_SHARES)]


def day_rows(usage, since, until):
    rows = []
    for offset in range((until - since).days + 1):
        day = (since + timedelta(days=offset)).isoformat()
        entries = [d for entry in usage for d in entry["daily"] if d["date"] == day]
        rows.append(row(day, None, sum(d["total_tokens"] for d in entries),
                        sum(d["cost_usd_micros"] for d in entries), 0))
    return rows


def with_share(rows, total_cost):
    return [{**entry, "share_permille": entry["cost_usd_micros"] * 1000 // total_cost if total_cost else 0}
            for entry in rows]


def grouped_rows(by, usage, since, until, today):
    if by == "day":
        return day_rows(usage, since, until)
    per_provider = {}
    for entry in sorted(usage, key=lambda e: PROVIDER_ORDER.index(e["provider"])):
        per_provider.setdefault(entry["provider"], []).append(range_totals(entry, since, until, today))
    rows = {"model": model_rows, "provider": provider_rows, "project": project_rows}[by](per_provider)
    return sorted(rows, key=lambda r: (-r["cost_usd_micros"], -r["tokens"]["total"], r["key"] or ""))


def spend_report(text, usage, today):
    query = json.loads(text)
    check_query(query)
    since, until = resolve_range(query, today)
    chosen = [entry for entry in usage if query.get("provider") in (None, entry["provider"])]
    rows = grouped_rows(query["by"], chosen, since, until, today)
    cost = sum(r["cost_usd_micros"] for r in rows)
    total = row(None, None, sum(r["tokens"]["total"] for r in rows), cost, sum(r["unpriced_tokens"] for r in rows))
    return {"since": since.isoformat(), "until": until.isoformat(), "by": query["by"],
            "rows": with_share(rows, cost), "total": with_share([total], cost)[0]}


def diagnostic_lines(accounts):
    return [f"  {index}. {entry['provider']} · {entry['status']} · {entry['source'] or 'none'} · {entry['owner']}"
            f" · updated {entry['updated_at'] or 'never'}" for index, entry in enumerate(accounts, 1)]


def diagnostics(accounts, settings, started_at, now):
    uptime = int((now - started_at).total_seconds())
    level = settings["logging"]["level"]
    providers = [{"provider": p, "accounts": sum(1 for e in accounts if e["provider"] == p), "usage_homes": 1}
                 for p in PROVIDER_ORDER if any(e["provider"] == p for e in accounts)]
    text = "\n".join([f"Headroom {APP_VERSION}", "OS: Mock Linux", "Desktop: GNOME (wayland)",
                      f"Uptime: {uptime // 3600}h {uptime % 3600 // 60}m", "IPC: dbus",
                      f"Log level: {level} (settings)", "Log file: ~/.local/state/headroom/headroom.log",
                      "Providers:", *[f"  {p['provider']}: {p['accounts']} account(s), 1 usage home(s)"
                                      for p in providers], "Accounts:", *diagnostic_lines(accounts)]) + "\n"
    return {
        "app_version": APP_VERSION, "os": "Mock Linux", "desktop": "GNOME (wayland)", "uptime_secs": uptime,
        "transports": ["dbus"], "log_level": level, "log_level_source": "settings",
        "log_file": "~/.local/state/headroom/headroom.log", "providers": providers,
        "accounts": [{"provider": e["provider"], "status": e["status"],
                      "error_kind": e["error"]["kind"] if e["error"] else None, "source": e["source"],
                      "owner": e["owner"], "hidden": e["hidden"], "updated_at": e["updated_at"]} for e in accounts],
        "text": text,
    }


def check_result(outcome, checked_at, now):
    if outcome == "rate_limited":
        return {"status": outcome, "checked_at": checked_at, "version": APP_VERSION, "until": iso(now + 30 * MINUTE)}
    version = "0.7.0" if outcome == "available" else APP_VERSION
    return {"status": outcome, "checked_at": checked_at, "version": version}
