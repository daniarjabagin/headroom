from mock_common import DAY
from mock_registry import PROVIDER_NAMES, PROVIDER_ORDER

USAGE_PERIODS = ("today", "yesterday", "last_30_days")
SPEND_PERIODS = ("today", "yesterday", "last_7_days", "last_30_days")
TOP_MODELS = 5
PROJECT_SHARES = [("~/code/headroom", 460), ("~/code/app", 270), (None, 110), ("~/work/api", 70),
                  ("~/code/dotfiles", 40), ("~/work/infra", 30), ("~/scratch", 20)]
TOP_PROJECTS = 5
MIN_LISTED_PERMILLE = 50
UNPRICED_PROJECT = "~/code/app"


def tokens(total):
    output = total // 20
    cache_read = total // 3
    return {"input": total - output - cache_read, "cache_read": cache_read, "cache_write": 0, "output": output,
            "reasoning": output // 4, "total": total}


def cost_per_mtok(cost_micros, priced_tokens):
    if priced_tokens <= 0:
        return None
    return (cost_micros * 1_000_000 + priced_tokens // 2) // priced_tokens


def model_usage(name, total_tokens, cost_micros, partial):
    return {"model": name, "total_tokens": total_tokens, "cost_usd_micros": cost_micros, "partial": partial,
            "cost_per_mtok_usd_micros": None if partial else cost_per_mtok(cost_micros, total_tokens)}


def by_cost(models):
    return sorted(models, key=lambda model: (-model["cost_usd_micros"], -model["total_tokens"], model["model"]))


def split_models(priced_tokens, cost_micros, mix):
    rows = [(name, priced_tokens * token_share // 100, cost_micros * cost_share // 100)
            for name, token_share, cost_share in mix]
    first_name, first_tokens, first_cost = rows[0]
    rows[0] = (first_name, first_tokens + priced_tokens - sum(row[1] for row in rows),
               first_cost + cost_micros - sum(row[2] for row in rows))
    return [model_usage(name, count, cost, False) for name, count, cost in rows if count > 0]


def totals_from(models):
    unpriced = [model for model in models if model["partial"]]
    total = sum(model["total_tokens"] for model in models)
    return {
        "tokens": tokens(total),
        "cost_usd_micros": sum(model["cost_usd_micros"] for model in models),
        "partial": bool(unpriced),
        "unpriced_tokens": sum(model["total_tokens"] for model in unpriced),
        "unpriced_models": sorted(model["model"] for model in unpriced),
        "models": by_cost(models),
    }


def totals(total_tokens, cost_micros, mix, unpriced=None):
    unpriced_tokens, unpriced_models = unpriced or (0, [])
    models = split_models(total_tokens - unpriced_tokens, cost_micros, mix)
    models += [model_usage(name, unpriced_tokens // len(unpriced_models), 0, True) for name in unpriced_models]
    return totals_from(models)


def scaled(entry, numerator, denominator):
    models = [model_usage(model["model"], model["total_tokens"] * numerator // denominator,
                          model["cost_usd_micros"] * numerator // denominator, model["partial"])
              for model in entry["models"]]
    return totals_from(models)


def period_totals(entry, period):
    return scaled(entry["last_30_days"], 7, 30) if period == "last_7_days" else entry[period]


def priced_tokens(models):
    return sum(model["total_tokens"] for model in models if not model["partial"])


def merge_models(groups):
    merged = {}
    for models in groups:
        for model in models:
            entry = merged.setdefault(model["model"], {"tokens": 0, "cost": 0, "partial": False})
            entry["tokens"] += model["total_tokens"]
            entry["cost"] += model["cost_usd_micros"]
            entry["partial"] = entry["partial"] or model["partial"]
    return by_cost(model_usage(name, v["tokens"], v["cost"], v["partial"]) for name, v in merged.items())


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
    if len(models) <= TOP_MODELS + 1:
        return {**entry, "models_other": None}
    return {**entry, "models": models[:TOP_MODELS], "models_other": models_other(models[TOP_MODELS:])}


def daily(now, seed, scale):
    days = []
    for offset in range(29, -1, -1):
        wave = (seed * (offset + 3) * 7919) % 100
        value = 0 if wave < 18 else wave * scale
        date = (now - offset * DAY).strftime("%Y-%m-%d")
        days.append({"date": date, "total_tokens": value, "cost_usd_micros": value * 3, "partial": False})
    return days


def usage_entry(provider, home, now, seed, periods, scale):
    entry = {"provider": provider, "provider_name": PROVIDER_NAMES[provider], "usage_home": home}
    entry.update(zip(USAGE_PERIODS, periods))
    entry["daily"] = daily(now, seed, scale)
    return entry


def published_usage(entry):
    return {**entry, **{period: with_top_models(entry[period]) for period in USAGE_PERIODS}}


def provider_rows(usage, period):
    rows = []
    for provider in sorted({entry["provider"] for entry in usage}, key=PROVIDER_ORDER.index):
        entries = [period_totals(entry, period) for entry in usage if entry["provider"] == provider]
        models = merge_models(t["models"] for t in entries)
        rows.append({
            "provider": provider,
            "provider_name": PROVIDER_NAMES[provider],
            "cost_usd_micros": sum(t["cost_usd_micros"] for t in entries),
            "total_tokens": sum(t["tokens"]["total"] for t in entries),
            "partial": any(t["partial"] for t in entries),
            "cost_per_mtok_usd_micros": cost_per_mtok(sum(t["cost_usd_micros"] for t in entries),
                                                      priced_tokens(models)),
            "models": models,
        })
    rows = [row for row in rows if row["cost_usd_micros"] or row["total_tokens"]]
    return sorted(rows, key=lambda row: (-row["cost_usd_micros"], row["provider_name"]))


def shares_of(value):
    parts = [value * share // 1000 for _, share in PROJECT_SHARES]
    parts[0] += value - sum(parts)
    return parts


def share_permille(entry, period_cost, period_tokens):
    if period_cost > 0:
        return entry["cost_usd_micros"] * 1000 // period_cost
    return entry["total_tokens"] * 1000 // period_tokens if period_tokens else 0


def unpriced_share(provider, row, count):
    if not provider["partial"] or row["project"] != UNPRICED_PROJECT:
        return 0
    return min(count, provider["total_tokens"] - priced_tokens(provider["models"]))


def project_rows(providers):
    rows = [{"project": project, "cost_usd_micros": 0, "total_tokens": 0, "priced_tokens": 0, "partial": False,
             "by_provider": []}
            for project, _ in PROJECT_SHARES]
    for provider in providers:
        costs, counts = shares_of(provider["cost_usd_micros"]), shares_of(provider["total_tokens"])
        for row, cost, count in zip(rows, costs, counts):
            row["cost_usd_micros"] += cost
            row["total_tokens"] += count
            row["priced_tokens"] += count - unpriced_share(provider, row, count)
            row["partial"] = row["partial"] or (provider["partial"] and row["project"] == UNPRICED_PROJECT)
            row["by_provider"].append({key: provider[key] for key in ("provider", "provider_name")}
                                      | {"cost_usd_micros": cost, "total_tokens": count})
    return sorted(rows, key=lambda row: (-row["cost_usd_micros"], -row["total_tokens"], row["project"] or ""))


def with_shares(rows, period_cost, period_tokens):
    return [{key: row[key] for key in ("project", "cost_usd_micros", "total_tokens", "partial")}
            | {"share_permille": share_permille(row, period_cost, period_tokens),
               "cost_per_mtok_usd_micros": cost_per_mtok(row["cost_usd_micros"], row["priced_tokens"]),
               "by_provider": row["by_provider"]}
            for row in rows]


def folded_projects(rest, period_cost, period_tokens):
    if not rest:
        return None
    other = {"count": len(rest), "cost_usd_micros": sum(row["cost_usd_micros"] for row in rest),
             "total_tokens": sum(row["total_tokens"] for row in rest),
             "partial": any(row["partial"] for row in rest)}
    rate = cost_per_mtok(other["cost_usd_micros"], sum(row["priced_tokens"] for row in rest))
    return {**other, "share_permille": share_permille(other, period_cost, period_tokens),
            "cost_per_mtok_usd_micros": rate}


def listed_count(rows, period_cost, period_tokens):
    listed = 0
    for row in rows[:TOP_PROJECTS]:
        if share_permille(row, period_cost, period_tokens) < MIN_LISTED_PERMILLE:
            break
        listed += 1
    return len(rows) if len(rows) - listed == 1 else listed


def period_spend(usage, period):
    rows = provider_rows(usage, period)
    cost = sum(row["cost_usd_micros"] for row in rows)
    total = sum(row["total_tokens"] for row in rows)
    projects = project_rows(rows) if rows else []
    listed = listed_count(projects, cost, total)
    return {
        "cost_usd_micros": cost,
        "total_tokens": total,
        "partial": any(row["partial"] for row in rows),
        "cost_per_mtok_usd_micros": cost_per_mtok(cost, sum(priced_tokens(row["models"]) for row in rows)),
        "by_provider": [with_top_models(row) for row in rows],
        "projects": with_shares(projects[:listed], cost, total),
        "projects_other": folded_projects(projects[listed:], cost, total),
    }


def spend(usage):
    return {period: period_spend(usage, period) for period in SPEND_PERIODS}
