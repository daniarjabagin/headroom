.pragma library

const GROUP = /^<([A-Za-z0-9]+)>/;
const FUNCTION_KEY = /^f([1-9]|[12]\d|3[0-5])$/;
const MODIFIER_ORDER = ["Ctrl", "Alt", "Shift", "Meta"];

const MODIFIERS = {
    control: "Ctrl",
    ctrl: "Ctrl",
    primary: "Ctrl",
    alt: "Alt",
    mod1: "Alt",
    shift: "Shift",
    super: "Meta",
    meta: "Meta",
    mod4: "Meta"
};

const KEYS = {
    return: "Return",
    kp_enter: "Enter",
    space: "Space",
    tab: "Tab",
    escape: "Esc",
    backspace: "Backspace",
    delete: "Del",
    insert: "Ins",
    home: "Home",
    end: "End",
    page_up: "PgUp",
    prior: "PgUp",
    page_down: "PgDown",
    next: "PgDown",
    left: "Left",
    right: "Right",
    up: "Up",
    down: "Down",
    print: "Print",
    pause: "Pause",
    menu: "Menu",
    comma: ",",
    period: ".",
    minus: "-",
    equal: "=",
    slash: "/",
    backslash: "\\",
    semicolon: ";",
    apostrophe: "'",
    grave: "`",
    bracketleft: "[",
    bracketright: "]"
};

function own(table, key) {
    return Object.prototype.hasOwnProperty.call(table, key) ? table[key] : null;
}

function splitGroups(accelerator) {
    const modifiers = [];
    let rest = accelerator;
    let match = GROUP.exec(rest);
    while (match !== null) {
        modifiers.push(match[1].toLowerCase());
        rest = rest.slice(match[0].length);
        match = GROUP.exec(rest);
    }
    return {
        modifiers,
        key: rest
    };
}

function keyName(key) {
    const lower = key.toLowerCase();
    if (/^[a-z0-9]$/.test(lower) || FUNCTION_KEY.test(lower))
        return lower.toUpperCase();
    return own(KEYS, lower);
}

function modifierNames(modifiers) {
    const names = modifiers.map(modifier => own(MODIFIERS, modifier));
    if (names.includes(null))
        return null;
    return MODIFIER_ORDER.filter(name => names.includes(name));
}

function keySequence(accelerator) {
    if (typeof accelerator !== "string" || accelerator === "")
        return "";
    const parts = splitGroups(accelerator);
    const key = keyName(parts.key);
    const modifiers = modifierNames(parts.modifiers);
    if (key === null || modifiers === null)
        return "";
    return modifiers.concat([key]).join("+");
}

function nextShortcut(accelerator, supported, applied) {
    if (supported !== true)
        return null;
    const sequence = keySequence(accelerator);
    return sequence === applied ? null : sequence;
}
