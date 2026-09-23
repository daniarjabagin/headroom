.pragma library

const PROVIDERS = {
    codex: {
        name: "Codex",
        icon: "openai.svg",
        tinted: true,
        ringColor: "#10A37F",
        signInCommand: "codex login"
    },
    claude: {
        name: "Claude Code",
        icon: "claude.svg",
        tinted: false,
        ringColor: "#DE7356",
        signInCommand: "claude"
    }
};

const FALLBACK_RING_COLORS = ["#34C759", "#5856D6", "#FF2D55", "#A2845E"];

function stableHash(value) {
    let hash = 0;
    for (const char of value)
        hash = (hash * 31 + char.codePointAt(0)) >>> 0;
    return hash;
}

function titleCase(id) {
    return id.charAt(0).toUpperCase() + id.slice(1);
}

function providerInfo(id) {
    return PROVIDERS[id] ?? {
        name: titleCase(id),
        icon: null,
        tinted: true,
        ringColor: FALLBACK_RING_COLORS[stableHash(id) % FALLBACK_RING_COLORS.length],
        signInCommand: null
    };
}

function accountName(account) {
    return account.label ?? account.email ?? providerInfo(account.provider).name;
}

function accountTitle(account, showName) {
    const name = providerInfo(account.provider).name;
    if (!showName)
        return name;
    const who = account.label ?? account.email;
    return who ? `${name}: ${who}` : name;
}
