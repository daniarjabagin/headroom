import { isQuietlyLimited } from '../accountStatus.js';
import { clockTime } from '../dates.js';
import { _, fill } from '../i18n.js';

export function rateLimitNote(account, hour12) {
    if (!isQuietlyLimited(account)) return null;
    const next = account.refresh?.nextAt ?? null;
    if (next === null) return _('Provider is limiting requests');
    return fill(_('Provider is limiting requests · next try {time}'), { time: clockTime(next, hour12) });
}
