import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

const STRING = String.raw`'((?:\\.|[^'\\])*)'|"((?:\\.|[^"\\])*)"`;
const PLAIN = new RegExp(String.raw`(?<![\w$])_\(\s*(?:${STRING})\s*\)`, 'g');
const CONTEXT = new RegExp(String.raw`(?<![\w$])C_\(\s*(?:${STRING})\s*,\s*(?:${STRING})\s*\)`, 'g');
const PLURAL = new RegExp(String.raw`(?<![\w$])n_\(\s*(?:${STRING})\s*,\s*(?:${STRING})\s*,`, 'g');
const ANY_CALL = /(?<![\w$])(?:_|C_|n_)\(/g;
const DEFINITION = /export function (?:_|C_|n_)\(/g;
const SKIPPED = ['locale', 'i18n.js'];

function unescape(text) {
    return text.replace(/\\(.)/g, '$1');
}

function literal(match, index) {
    return unescape(match[index] ?? match[index + 1]);
}

function sourceFiles(dir, found = []) {
    const children = dir.enumerate_children('standard::name,standard::type', Gio.FileQueryInfoFlags.NONE, null);
    for (let info = children.next_file(null); info; info = children.next_file(null)) {
        const child = dir.get_child(info.get_name());
        if (SKIPPED.includes(info.get_name())) continue;
        if (info.get_file_type() === Gio.FileType.DIRECTORY) sourceFiles(child, found);
        else if (info.get_name().endsWith('.js')) found.push(child);
    }
    return found;
}

function read(file) {
    const [, bytes] = GLib.file_get_contents(file.get_path());
    return new TextDecoder().decode(bytes);
}

function scanText(text, messages, name) {
    const plain = [...text.matchAll(PLAIN)].map(match => literal(match, 1));
    const context = [...text.matchAll(CONTEXT)].map(match => [literal(match, 1), literal(match, 3)]);
    const plural = [...text.matchAll(PLURAL)].map(match => literal(match, 1));
    const calls = [...text.matchAll(ANY_CALL)].length - [...text.matchAll(DEFINITION)].length;
    if (calls !== plain.length + context.length + plural.length) messages.dynamic.push(name);
    messages.plain.push(...plain);
    messages.context.push(...context);
    messages.plural.push(...plural);
}

export function scanMessages(extensionDir) {
    const messages = { plain: [], context: [], plural: [], dynamic: [] };
    const files = [
        ...sourceFiles(extensionDir.get_child('src')),
        extensionDir.get_child('prefs.js'),
        extensionDir.get_child('extension.js'),
    ];
    for (const file of files) scanText(read(file), messages, file.get_basename());
    return {
        plain: [...new Set(messages.plain)],
        context: [...new Map(messages.context.map(entry => [entry.join('\u0004'), entry])).values()],
        plural: [...new Set(messages.plural)],
        dynamic: messages.dynamic,
    };
}
