.pragma library

const ICONS = ["antigravity", "claude", "cline", "codex", "copilot", "cursor", "kimi", "minimax", "ollama", "opencode", "openrouter", "zai"];
const BRANDED = ["claude"];
const GENERIC_ICON = "provider-generic.svg";

const COLORS = {
    codex: ["#10A37F", "#19C37D"],
    claude: ["#D97757", "#D97757"],
    opencode: ["#0284C7", "#38BDF8"],
    openrouter: ["#6366F1", "#818CF8"],
    zai: ["#9333EA", "#C084FC"],
    kimi: ["#DB2777", "#F472B6"],
    minimax: ["#E11D48", "#FB7185"],
    grok: ["#475569", "#94A3B8"],
    cline: ["#CA8A04", "#FACC15"],
    devin: ["#2563EB", "#60A5FA"],
    copilot: ["#0D9488", "#2DD4BF"],
    cursor: ["#65A30D", "#A3E635"],
    antigravity: ["#A2845E", "#C4A484"],
    ollama: ["#78716C", "#A8A29E"]
};

const FALLBACK_COLORS = [["#0891B2", "#22D3EE"], ["#C026D3", "#E879F9"], ["#EA580C", "#FB923C"], ["#4F46E5", "#A5B4FC"], ["#059669", "#34D399"]];

function stableHash(value) {
    let hash = 0;
    for (const char of value)
        hash = (hash * 31 + char.codePointAt(0)) >>> 0;
    return hash;
}

function hasIcon(id) {
    return ICONS.includes(id);
}

function iconFile(id) {
    return hasIcon(id) ? `${id}.svg` : GENERIC_ICON;
}

function isTinted(id) {
    return !BRANDED.includes(id);
}

function colorPair(id) {
    return COLORS[id] ?? FALLBACK_COLORS[stableHash(id) % FALLBACK_COLORS.length];
}

function ringColor(id, dark) {
    return colorPair(id)[dark ? 1 : 0];
}

function accountName(account) {
    return account.label ?? account.email ?? account.providerName;
}

function accountTitle(account, showName) {
    if (!showName)
        return account.providerName;
    const who = account.label ?? account.email;
    return who ? `${account.providerName}: ${who}` : account.providerName;
}
