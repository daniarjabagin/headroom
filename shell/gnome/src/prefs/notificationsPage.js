import Adw from 'gi://Adw';
import { _ } from '../i18n.js';
import { notificationsPatch } from '../settings.js';
import { group, switchRow } from './rows.js';

const MILESTONES = [
    ['almostOut', () => _('Almost out'), () => _('A limit drops under 10% left')],
    ['cuttingItClose', () => _('Cutting it close'), () => _('The pace says a limit will barely last until reset')],
    ['willRunOut', () => _('Will run out'), () => _('The pace says a limit runs out before it resets')],
    ['reset', () => _('Limit reset'), () => _('A limit that was running low resets')],
];

export class NotificationsPage {
    constructor(client) {
        this.page = new Adw.PreferencesPage({
            title: _('Notifications'),
            icon_name: 'preferences-system-notifications-symbolic',
        });
        this._rows = MILESTONES.map(([key, title, subtitle]) =>
            switchRow({
                title: title(),
                subtitle: subtitle(),
                onChange: value => client.updateSettings(notificationsPatch({ [key]: value })),
            })
        );
        const description = _('Hidden accounts and hidden limits never notify.');
        this.page.add(
            group(
                _('Notify Me When'),
                this._rows.map(entry => entry.row),
                description
            )
        );
    }

    update(settings) {
        MILESTONES.forEach(([key], index) => this._rows[index].set(settings.notifications[key]));
    }
}
