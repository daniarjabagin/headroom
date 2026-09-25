import { agoText, duration } from '../format.js';
import { _, fill } from '../i18n.js';

const MINUTE = 60 * 1000;

const KINDS = {
    minor: () => _('Incident'),
    maintenance: () => _('Maintenance'),
    major: () => _('Partial outage'),
    critical: () => _('Major outage'),
};

const STAGES = {
    investigating: () => _('Investigating — requests may fail or be slow.'),
    identified: () => _('Identified — the cause is known and a fix is under way.'),
    monitoring: () => _('Monitoring — a fix is out and being watched.'),
    in_progress: () => _('In progress — work is under way.'),
    verifying: () => _('Verifying — checking that the work is done.'),
};

export function statusKind(status) {
    return (KINDS[status.indicator] ?? KINDS.minor)();
}

export function statusTone(status) {
    return status.tone === 'critical' ? 'error' : 'warning';
}

export function statusTitle(status) {
    const kind = statusKind(status);
    return status.title ? `${kind} · ${status.title}` : kind;
}

export function statusDetail(status) {
    return STAGES[status.stage]?.() ?? null;
}

export function statusStarted(status, now) {
    if (status.startedAt === null) return null;
    return fill(_('Started {ago}'), { ago: agoText(status.startedAt, now) });
}

export function statusAge(status, now) {
    if (status.startedAt === null) return null;
    return `· ${duration(Math.max(MINUTE, now - status.startedAt))}`;
}

export function statusIcon(status) {
    return status.tone === 'critical' ? 'dialog-error-symbolic' : 'dialog-warning-symbolic';
}
