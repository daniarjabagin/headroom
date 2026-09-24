import { _, fill } from './i18n.js';

const INSTALL_KINDS = new Set(['self', 'package', 'unknown']);
const RELEASE_URL = /^https:\/\/\S+$/;

function isObject(value) {
    return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function text(value) {
    return typeof value === 'string' && value.trim().length > 0 ? value.trim() : null;
}

function releaseUrl(value) {
    const url = text(value);
    return url && RELEASE_URL.test(url) ? url : null;
}

function publishedAt(value) {
    const ms = typeof value === 'string' ? Date.parse(value) : NaN;
    return Number.isNaN(ms) ? null : new Date(ms);
}

export function parseUpdate(raw) {
    if (!isObject(raw)) return null;
    const version = text(raw.version);
    if (!version) return null;
    return {
        version,
        url: releaseUrl(raw.url),
        publishedAt: publishedAt(raw.published_at),
        install: INSTALL_KINDS.has(raw.install) ? raw.install : 'unknown',
        command: text(raw.command),
    };
}

export function updateAction(update) {
    if (update.install === 'self') return { kind: 'install', label: _('Update') };
    if (update.install === 'package' && update.command) return { kind: 'command', label: _('How to update') };
    if (update.url) return { kind: 'notes', label: _('Release notes') };
    return null;
}

export function showsWhatsNew(update) {
    return update.url !== null && updateAction(update)?.kind !== 'notes';
}

export function updateTitle(update) {
    return fill(_('Headroom {version} is available'), { version: update.version });
}

export const IDLE = Object.freeze({ phase: 'idle' });

export function startedRun() {
    return { phase: 'running', step: null };
}

export function afterEvent(run, event) {
    if (run.phase !== 'running') return run;
    if (event.event === 'step') return { phase: 'running', step: event.text };
    if (event.event === 'done') return { phase: 'done', version: event.version, relogin: event.relogin };
    if (event.event === 'error') return { phase: 'failed', message: event.message };
    return run;
}

export function afterExit(run, errorMessage) {
    if (run.phase !== 'running') return run;
    return { phase: 'failed', message: errorMessage ?? _('The update stopped before it finished') };
}

export function runLine(run) {
    switch (run.phase) {
        case 'running':
            return run.step ?? _('Starting the update…');
        case 'done':
            if (run.relogin) return _('Updated — log out and back in to finish');
            return run.version ? fill(_('Updated to {version}'), { version: run.version }) : _('Updated');
        case 'failed':
            return run.message;
        default:
            return null;
    }
}
