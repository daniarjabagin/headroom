.pragma library

const ICONS = ["antigravity", "claude", "cline", "codex", "copilot", "cursor", "deepseek", "kimi", "minimax", "moonshot", "ollama", "opencode", "openrouter", "poe", "warp", "zai"];
const BRANDED = ["claude"];
const GENERIC_ICON = "provider-generic.svg";

const COLORS = {
    codex: ["#10A37F", "#10A37F"],
    claude: ["#D97757", "#D97757"],
    opencode: ["#6E6E73", "#AEAEB2"],
    openrouter: ["#6467F2", "#7C7FF5"],
    zai: ["#2D2D2D", "#D1D1D6"],
    kimi: ["#1D93D2", "#3AA9E4"],
    minimax: ["#E73562", "#F0527A"],
    grok: ["#8E8E93", "#98989D"],
    cline: ["#0F9D9A", "#2CC3BF"],
    devin: ["#8B5E3C", "#B08560"],
    copilot: ["#A855F7", "#B77CF9"],
    cursor: ["#13120A", "#F5F5F7"],
    antigravity: ["#4285F4", "#5B96F6"],
    ollama: ["#65A30D", "#84CC16"],
    kilo: ["#B59A00", "#F8F675"],
    warp: ["#005A9E", "#6CCBFF"],
    poe: ["#B42BC9", "#DE7BF0"],
    deepseek: ["#2C3FC2", "#A3B1FF"],
    moonshot: ["#475A78", "#A5B4CC"]
};

const FALLBACK_COLORS = [["#34C759", "#30D158"], ["#5856D6", "#5E5CE6"], ["#FF2D55", "#FF375F"], ["#A2845E", "#AC8E68"]];

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
