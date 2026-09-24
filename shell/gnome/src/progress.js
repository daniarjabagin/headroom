function decode(line) {
    try {
        const value = JSON.parse(line);
        return value !== null && typeof value === 'object' && !Array.isArray(value) ? value : null;
    } catch {
        return null;
    }
}

function text(value) {
    return typeof value === 'string' && value.length > 0 ? value : null;
}

function known(raw) {
    switch (raw.event) {
        case 'started':
            return { event: 'started' };
        case 'url':
            return text(raw.url) && { event: 'url', url: raw.url };
        case 'output':
            return typeof raw.line === 'string' ? { event: 'output', line: raw.line } : null;
        case 'step':
            return text(raw.text) && { event: 'step', text: raw.text };
        case 'done':
            return {
                event: 'done',
                accountId: text(raw.account_id),
                version: text(raw.version),
                relogin: raw.relogin === true,
            };
        case 'error':
            return { event: 'error', message: text(raw.message) ?? 'unknown error' };
        default:
            return null;
    }
}

export function parseProgressLine(line) {
    if (line.trim().length === 0) return null;
    const raw = decode(line);
    return (raw && known(raw)) || { event: 'output', line };
}
