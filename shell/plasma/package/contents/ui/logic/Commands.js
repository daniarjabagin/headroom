.pragma library

const PROVIDER_ID = /^[a-z0-9_-]+$/;
const TERMINAL_METHODS = ["cli_login", "api_key"];
const ACCOUNT_ID = /^[a-z0-9_-]+:[0-9a-f]+$/;

class CommandError extends Error {}

function shellQuote(value) {
    return `'${String(value).replace(/'/g, "'\\''")}'`;
}

function loginScript(provider, label, closePrompt) {
    const labelPart = label.trim() === "" ? "" : ` --label ${shellQuote(label.trim())}`;
    const add = `headroom accounts add ${provider}${labelPart}`;
    return `${add}; status=$?; printf '\\n%s ' ${shellQuote(closePrompt)}; read -r _; exit $status`;
}

function addAccountCommand(provider, label, closePrompt) {
    if (!PROVIDER_ID.test(provider))
        throw new CommandError(`Unexpected provider id ${provider}`);
    const script = shellQuote(loginScript(provider, label, closePrompt));
    return `if command -v xdg-terminal-exec >/dev/null 2>&1; then exec xdg-terminal-exec sh -c ${script}; else exec konsole -e sh -c ${script}; fi`;
}

function addPlan(provider, label, closePrompt) {
    if (TERMINAL_METHODS.includes(provider.method.kind))
        return {
            kind: "terminal",
            command: addAccountCommand(provider.id, label, closePrompt)
        };
    return {
        kind: "rescan",
        command: ""
    };
}

function removeAccountCommand(accountId) {
    if (!ACCOUNT_ID.test(accountId))
        throw new CommandError(`Unexpected account id ${accountId}`);
    return `headroom accounts remove ${shellQuote(accountId)} --yes --progress json`;
}

function parseEvent(line) {
    try {
        const event = JSON.parse(line);
        return event !== null && typeof event === "object" ? event : null;
    } catch (error) {
        return null;
    }
}

function progressOutcome(stdout, exitCode) {
    const events = String(stdout ?? "").split("\n").map(parseEvent).filter(event => event !== null);
    const last = events[events.length - 1] ?? null;
    if (exitCode === 0 && last?.event === "done")
        return {
            ok: true,
            message: ""
        };
    const failure = events.find(event => event.event === "error");
    return {
        ok: false,
        message: typeof failure?.message === "string" ? failure.message : ""
    };
}

function isCommandError(error) {
    return error instanceof CommandError;
}
