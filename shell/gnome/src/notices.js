import { _, fill } from './i18n.js';

const KIND_BY_TONE = { critical: 'error', warning: 'warning' };

const FIXED_TEXTS = () => ({
    'Weekly limit shared with Codex Cloud': _('Weekly limit shared with Codex Cloud'),
    'Offline — showing limits from local logs.': _('Offline — showing limits from local logs.'),
    'Sign-in expired — open Codex to sign in again. Showing limits from local logs.': _(
        'Sign-in expired — open Codex to sign in again. Showing limits from local logs.'
    ),
    'Credit balance needs a management key': _('Credit balance needs a management key'),
    'Credit balance is unavailable right now': _('Credit balance is unavailable right now'),
    'No Cline credits left.': _('No Cline credits left.'),
    'Legacy Grok billing has no weekly pool.': _('Legacy Grok billing has no weekly pool.'),
    'Ollama reports no Cloud limits for this account yet.': _('Ollama reports no Cloud limits for this account yet.'),
    'Could not read the Ollama plan; the usage above is up to date.': _(
        'Could not read the Ollama plan; the usage above is up to date.'
    ),
    'Kilo credits are used up': _('Kilo credits are used up'),
    'Unlimited credits': _('Unlimited credits'),
    'No monthly credits on this plan': _('No monthly credits on this plan'),
    'Balance is not enough for API calls': _('Balance is not enough for API calls'),
    'Balance is used up; API calls fail until you top up': _('Balance is used up; API calls fail until you top up'),
    'Balance is used up; API requests fail until you top up': _(
        'Balance is used up; API requests fail until you top up'
    ),
    'Cash balance is negative: the account is in debt': _('Cash balance is negative: the account is in debt'),
    'Antigravity reports no quota pools for this account.': _('Antigravity reports no quota pools for this account.'),
    'The keyring that holds the Antigravity sign-in is locked. Unlock it or start Antigravity.': _(
        'The keyring that holds the Antigravity sign-in is locked. Unlock it or start Antigravity.'
    ),
});

const PATTERNS = [
    [/^Extra usage on, cap (.+)$/, ([, amount]) => fill(_('Extra usage on, cap {amount}'), { amount })],
    [/^(.+) renews on (\S+) \(UTC\)\.$/, ([, plan, date]) => fill(_('{plan} renews on {date} (UTC).'), { plan, date })],
    [/^(.+) ends on (\S+) \(UTC\)\.$/, ([, plan, date]) => fill(_('{plan} ends on {date} (UTC).'), { plan, date })],
];

export function noticeKind(tone) {
    return KIND_BY_TONE[tone] ?? 'info';
}

export function noticeText(text) {
    const fixed = FIXED_TEXTS();
    if (Object.hasOwn(fixed, text)) return fixed[text];
    for (const [pattern, translate] of PATTERNS) {
        const match = pattern.exec(text);
        if (match) return translate(match);
    }
    return text;
}
