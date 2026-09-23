export function _(msgid) {
    return msgid;
}

export function fill(template, values) {
    return template.replace(/\{(\w+)\}/g, (match, name) => (name in values ? String(values[name]) : match));
}
