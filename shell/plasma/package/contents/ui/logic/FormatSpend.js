.pragma library

.import "FormatTime.js" as FormatTime
.import "I18n.js" as I18n
.import "Money.js" as Money

const DASH = "—";
const ELLIPSIS = "…";
const UNITS = [[1e12, "T"], [1e9, "B"], [1e6, "M"], [1e3, "K"]];

function tokenDigits(scaled) {
    return scaled >= 100 ? 0 : 1;
}

function moneyDigits(scaled) {
    if (scaled >= 100)
        return 0;
    return scaled >= 10 ? 1 : 2;
}

function abbreviate(value, digitsFor) {
    for (const [size, suffix] of UNITS) {
        if (value >= size) {
            const scaled = value / size;
            const digits = scaled.toFixed(digitsFor(scaled));
            return `${digits.replace(/\.0+$/, "")}${suffix}`;
        }
    }
    return String(value);
}

function compactTokens(count) {
    return abbreviate(count, tokenDigits);
}

function exactTokens(count) {
    return String(count).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}

function tokenCount(lang, count) {
    return I18n.trn(lang, "{tokens} token", "{tokens} tokens", count, {
        tokens: exactTokens(count)
    });
}

function tokensText(lang, count) {
    return I18n.tr(lang, "{tokens} tokens", {
        tokens: compactTokens(count)
    });
}

function exactUsd(micros) {
    return Money.money("USD", micros);
}

function usd(micros) {
    const cents = Money.centsOf(micros);
    if (cents >= 100000)
        return `$${abbreviate(cents / 100, moneyDigits)}`;
    return exactUsd(micros);
}

function ringUsd(micros) {
    const cents = Money.centsOf(micros);
    if (cents < 10000)
        return usd(micros);
    if (cents < 1000000)
        return `$${Math.round(cents / 100)}`;
    return usd(micros);
}

function costPerMtok(lang, micros) {
    if (micros === null)
        return DASH;
    return I18n.tr(lang, "{cost}/MTok", {
        cost: usd(micros)
    });
}

function shareText(lang, permille) {
    const separator = lang === "ru" ? "," : ".";
    return `${Math.floor(permille / 10)}${separator}${permille % 10}%`;
}

function unitValue(totals, unit) {
    if (unit === "tokens")
        return compactTokens(totals.totalTokens);
    if (unit === "cost_per_mtok")
        return totals.costPerMtokMicros === null ? DASH : usd(totals.costPerMtokMicros);
    return ringUsd(totals.costMicros);
}

function spendLine(lang, totals) {
    if (totals.costMicros === 0 && totals.totalTokens === 0)
        return I18n.tr(lang, "No data");
    return I18n.tr(lang, "{cost} · {tokens} tokens", {
        cost: usd(totals.costMicros),
        tokens: compactTokens(totals.totalTokens)
    });
}

function spendTooltip(lang, totals) {
    if (totals.totalTokens === 0)
        return "";
    const partial = totals.partial ? I18n.tr(lang, " · some models unpriced") : "";
    return `${exactUsd(totals.costMicros)} · ${tokenCount(lang, totals.totalTokens)}${partial}`;
}

function balanceValue(lang, balance) {
    if (balance.kind === "usd" && balance.usdMicros !== null)
        return usd(balance.usdMicros);
    if (balance.kind === "money" && balance.currency !== null && balance.micros !== null)
        return Money.money(balance.currency, balance.micros);
    if (balance.kind === "count" && balance.value !== null)
        return `${exactTokens(balance.value)} ${balance.unit ?? ""}`.trim();
    return I18n.tr(lang, "No data");
}

function dayTooltip(lang, day) {
    if (day.date === "")
        return "";
    const date = FormatTime.dayText(lang, day.date);
    if (day.totalTokens === 0)
        return I18n.tr(lang, "{date} · no usage", {
            date
        });
    return I18n.tr(lang, "{date} · {tokens} tokens · {cost}", {
        date,
        tokens: compactTokens(day.totalTokens),
        cost: usd(day.costMicros)
    });
}

function middleEllipsis(value, maxChars) {
    const chars = Array.from(value);
    if (chars.length <= maxChars)
        return value;
    const slash = value.lastIndexOf("/");
    const tail = Array.from(slash >= 0 ? value.slice(slash) : "");
    const kept = maxChars - 1;
    if (tail.length === 0 || tail.length >= kept) {
        const head = Math.ceil(kept / 2);
        return `${chars.slice(0, head).join("")}${ELLIPSIS}${chars.slice(chars.length - (kept - head)).join("")}`;
    }
    return `${chars.slice(0, kept - tail.length).join("")}${ELLIPSIS}${tail.join("")}`;
}

function projectLabel(lang, project, maxChars) {
    if (project === null)
        return I18n.tr(lang, "No project");
    return middleEllipsis(project, maxChars);
}
