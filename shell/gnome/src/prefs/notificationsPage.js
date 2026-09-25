import Adw from 'gi://Adw';
import { _ } from '../i18n.js';
import { notificationsPatch, supports06 } from '../settings.js';
import { almostOutSubtitle } from './notificationsModel.js';
import { QuietHoursRows } from './quietHoursRows.js';
import { group, switchRow } from './rows.js';
import { ThresholdRows } from './thresholdRows.js';

const MILESTONES = [
    ['cuttingItClose', () => _('Cutting it close'), () => _('The pace says a limit will barely last until reset')],
    ['willRunOut', () => _('Will run out'), () => _('The pace says a limit runs out before it resets')],
    ['reset', () => _('Limit reset'), () => _('A limit that was running low resets')],
];

export class NotificationsPage {
    constructor(client, dir) {
        this.page = new Adw.PreferencesPage({
            title: _('Notifications'),
            icon_name: 'preferences-system-notifications-symbolic',
        });
        const patch = key => value => client.updateSettings(notificationsPatch({ [key]: value }));
        this._almostOut = switchRow({
            title: _('Almost out'),
            subtitle: almostOutSubtitle(10),
            onChange: patch('almostOut'),
        });
        this._rows = MILESTONES.map(([key, title, subtitle]) =>
            switchRow({ title: title(), subtitle: subtitle(), onChange: patch(key) })
        );
        this._threshold = new ThresholdRows(client, dir);
        this._quietHours = new QuietHoursRows(client);
        const description = _('Hidden accounts and hidden limits never notify.');
        this.page.add(
            group(_('Notify Me When'), [this._almostOut.row, ...this._rows.map(entry => entry.row)], description)
        );
        this.page.add(this._threshold.group);
        this.page.add(this._quietHours.group);
    }

    update(settings, state) {
        const notifications = settings.notifications;
        this._almostOut.set(notifications.almostOut);
        this._almostOut.row.subtitle = almostOutSubtitle(notifications.thresholdPercent);
        MILESTONES.forEach(([key], index) => this._rows[index].set(notifications[key]));
        const supported = supports06(state);
        this._threshold.group.visible = supported;
        this._quietHours.group.visible = supported;
        if (!supported) return;
        this._threshold.update(settings, state);
        this._quietHours.update(settings);
    }

    destroy() {
        this._quietHours.destroy();
    }
}
