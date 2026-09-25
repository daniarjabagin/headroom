import { _, fill } from '../i18n.js';

export const SEPARATOR = 'separator';

const LINK_ITEMS = [
    ['status', 'status', () => _('Status page')],
    ['dashboard', 'external', () => _('Open dashboard')],
    ['usage', 'chart', () => _('Usage page')],
];

export function hostOf(url) {
    const host = url.replace(/^https:\/\//, '').split(/[/?#]/)[0];
    return host.replace(/^www\./, '');
}

export function shownLinks(links) {
    if (!links) return [];
    return LINK_ITEMS.filter(([key]) => links[key])
        .filter(([key]) => key !== 'usage' || links.usage !== links.dashboard)
        .map(([key, icon, title]) => ({ key, icon, title: title(), url: links[key], host: hostOf(links[key]) }));
}

function accountItems({ providerName, canHide, canStar, starred }) {
    const items = [
        { id: 'refresh', icon: 'view-refresh', label: fill(_('Refresh {provider}'), { provider: providerName }) },
    ];
    if (canHide) items.push({ id: 'hide', icon: 'view-conceal', label: _('Hide from popup') });
    if (canStar)
        items.push({
            id: 'star',
            icon: starred ? 'non-starred' : 'starred',
            label: starred ? _('Show on demand') : _('Always show'),
        });
    return items;
}

function linkItems(links) {
    return shownLinks(links).map(link => ({
        id: `link:${link.key}`,
        icon: link.icon,
        label: link.title,
        accel: link.host,
        url: link.url,
    }));
}

function shareItems(canShare) {
    if (!canShare) return [];
    return [
        { id: 'share', icon: 'share', label: _('Share as image…') },
        { id: 'copy', icon: 'edit-copy', label: _('Copy as text') },
    ];
}

export function cardMenuItems(options) {
    const groups = [accountItems(options), linkItems(options.links), shareItems(options.canShare)];
    return groups
        .filter(group => group.length > 0)
        .flatMap((group, index) => (index === 0 ? group : [SEPARATOR, ...group]));
}
