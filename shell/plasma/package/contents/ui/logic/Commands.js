.pragma library

.import "I18n.js" as I18n

const PROVIDER_ID = /^[a-z0-9][a-z0-9_-]*$/;
const TERMINAL_METHODS = ["cli_login", "api_key"];
const ACCOUNT_ID = /^[a-z0-9][a-z0-9_-]*:[0-9a-f]+$/;

class CommandError extends I18n.LocalizedError {}

function shellQuote(value) {
    return `'${String(value).replace(/'/g, "'\\''")}'`;
}

function terminalScript(command, closePrompt) {
    return `${command}; status=$?; printf '\\n%s ' ${shellQuote(closePrompt)}; read -r _; exit $status`;
}

function loginScript(provider, label, closePrompt) {
    const labelPart = label.trim() === "" ? "" : ` --label=${shellQuote(label.trim())}`;
    return terminalScript(`headroom accounts add ${provider}${labelPart}`, closePrompt);
}

function inTerminal(shellScript) {
    const script = shellQuote(shellScript);
    return `if command -v xdg-terminal-exec >/dev/null 2>&1; then exec xdg-terminal-exec sh -c ${script}; else exec konsole -e sh -c ${script}; fi`;
}

function addAccountCommand(provider, label, closePrompt) {
    if (!PROVIDER_ID.test(provider))
        throw new CommandError(I18n.N("Unexpected provider id {provider}"), {
            provider
        });
    return inTerminal(loginScript(provider, label, closePrompt));
}

function loginAccountCommand(accountId, closePrompt) {
    if (!ACCOUNT_ID.test(accountId))
        throw new CommandError(I18n.N("Unexpected account id {account}"), {
            account: accountId
        });
    return inTerminal(terminalScript(`headroom accounts login ${shellQuote(accountId)}`, closePrompt));
}

function signInCommand(target, closePrompt) {
    return ACCOUNT_ID.test(target) ? loginAccountCommand(target, closePrompt) : addAccountCommand(target, "", closePrompt);
}

function opensTerminal(method) {
    return TERMINAL_METHODS.includes(method?.kind);
}

function addPlan(provider, label, closePrompt) {
    if (opensTerminal(provider.method))
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
        throw new CommandError(I18n.N("Unexpected account id {account}"), {
            account: accountId
        });
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

function track(pending, command, kind) {
    return Object.assign({}, pending, {
        [command]: kind
    });
}

function settle(pending, command) {
    const rest = Object.assign({}, pending);
    delete rest[command];
    return {
        kind: Object.prototype.hasOwnProperty.call(pending, command) ? pending[command] : "",
        pending: rest
    };
}

function isCommandError(error) {
    return error instanceof CommandError;
}
