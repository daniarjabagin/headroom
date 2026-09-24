.pragma library

.import "I18n.js" as I18n

const LABELS = [I18n.N("Credits"), I18n.N("Extra usage"), I18n.N("Balance"), I18n.N("Vouchers"), I18n.N("Cash"), I18n.N("Credit balance"), I18n.N("Organization credits"), I18n.N("Point balance"), I18n.N("Bonus credits"), I18n.N("Monthly credits")];

const NOTICES = [
    I18n.N("Weekly limit shared with Codex Cloud"),
    I18n.N("Offline — showing limits from local logs."),
    I18n.N("Sign-in expired — open Codex to sign in again. Showing limits from local logs."),
    I18n.N("Credit balance needs a management key"),
    I18n.N("Credit balance is unavailable right now"),
    I18n.N("No Cline credits left."),
    I18n.N("Legacy Grok billing has no weekly pool."),
    I18n.N("Ollama reports no Cloud limits for this account yet."),
    I18n.N("Could not read the Ollama plan; the usage above is up to date."),
    I18n.N("Antigravity reports no quota pools for this account."),
    I18n.N("The keyring that holds the Antigravity sign-in is locked. Unlock it or start Antigravity."),
    I18n.N("Kilo credits are used up"),
    I18n.N("Unlimited credits"),
    I18n.N("No monthly credits on this plan"),
    I18n.N("Balance is not enough for API calls"),
    I18n.N("Balance is used up; API calls fail until you top up"),
    I18n.N("Balance is used up; API requests fail until you top up"),
    I18n.N("Cash balance is negative: the account is in debt")
];

const NOTICE_PATTERNS = [
    [/^Extra usage on, cap (.+)$/, I18n.N("Extra usage on, cap {amount}"), match => ({
                amount: match[1]
            })],
    [/^(.+) renews on (\S+) \(UTC\)\.$/, I18n.N("{plan} renews on {date} (UTC)."), match => ({
                plan: match[1],
                date: match[2]
            })],
    [/^(.+) ends on (\S+) \(UTC\)\.$/, I18n.N("{plan} ends on {date} (UTC)."), match => ({
                plan: match[1],
                date: match[2]
            })]
];

function exact(lang, known, text) {
    return known.includes(text) ? I18n.tr(lang, text) : null;
}

function patterned(lang, text) {
    for (const [pattern, msgid, values] of NOTICE_PATTERNS) {
        const match = pattern.exec(text);
        if (match)
            return I18n.tr(lang, msgid, values(match));
    }
    return null;
}

function label(lang, text) {
    return exact(lang, LABELS, text) ?? text;
}

function notice(lang, text) {
    return exact(lang, NOTICES, text) ?? patterned(lang, text) ?? text;
}
