import { _ } from '../i18n.js';

export const REPOSITORY_URL = 'https://github.com/daniarjabagin/headroom';

export function supportCopy() {
    return {
        group: _('Support Headroom'),
        title: _('Star Headroom on GitHub'),
        subtitle: _("Stars help other people find it. It's free and takes a second."),
        action: _('Open GitHub'),
    };
}
