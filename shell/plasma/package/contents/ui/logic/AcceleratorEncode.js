.pragma library

.import "SettingsValues.js" as Values

const MODIFIERS = [["Shift", "<Shift>"], ["Ctrl", "<Control>"], ["Alt", "<Alt>"], ["Meta", "<Super>"]];
const MODIFIER_ALIASES = {
    Shift: "Shift",
    Ctrl: "Ctrl",
    Control: "Ctrl",
    Alt: "Alt",
    Meta: "Meta",
    Super: "Meta"
};
const NAMED_KEYS = {
    Space: "space",
    Return: "Return",
    Enter: "KP_Enter",
    Tab: "Tab",
    Backspace: "BackSpace",
    Del: "Delete",
    Delete: "Delete",
    Ins: "Insert",
    Insert: "Insert",
    Home: "Home",
    End: "End",
    PgUp: "Page_Up",
    PgDown: "Page_Down",
    Left: "Left",
    Right: "Right",
    Up: "Up",
    Down: "Down",
    Esc: "Escape",
    Print: "Print",
    Pause: "Pause",
    Menu: "Menu",
    "+": "plus",
    ",": "comma",
    ".": "period",
    "/": "slash",
    ";": "semicolon",
    "'": "apostrophe",
    "[": "bracketleft",
    "]": "bracketright",
    "\\": "backslash",
    "-": "minus",
    "=": "equal",
    "`": "grave"
};
const LETTER_OR_DIGIT = /^[A-Za-z0-9]$/;
const FUNCTION_KEY = /^F([1-9]|[12]\d|3[0-5])$/;

function own(table, key) {
    return Object.prototype.hasOwnProperty.call(table, key) ? table[key] : null;
}

function splitChord(text) {
    if (text.endsWith("++"))
        return text.slice(0, -2).split("+").filter(part => part !== "").concat(["+"]);
    return text === "+" ? ["+"] : text.split("+");
}

function keyName(key) {
    if (LETTER_OR_DIGIT.test(key))
        return key.toLowerCase();
    if (FUNCTION_KEY.test(key))
        return key;
    return own(NAMED_KEYS, key);
}

function modifierPrefix(parts) {
    const names = parts.map(part => own(MODIFIER_ALIASES, part));
    if (names.includes(null))
        return null;
    return MODIFIERS.filter(([name]) => names.includes(name)).map(([, gtk]) => gtk).join("");
}

function encode(text) {
    const chord = String(text ?? "").trim();
    if (chord === "")
        return "";
    if (chord.includes(", "))
        return null;
    const parts = splitChord(chord);
    const key = keyName(parts[parts.length - 1]);
    const prefix = modifierPrefix(parts.slice(0, -1));
    if (key === null || prefix === null)
        return null;
    const accelerator = `${prefix}${key}`;
    return Values.isShortcut(accelerator) ? accelerator : null;
}
