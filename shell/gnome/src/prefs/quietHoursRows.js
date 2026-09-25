import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import { usesHour12 } from '../dates.js';
import { DesktopClock } from '../desktopClock.js';
import { _ } from '../i18n.js';
import { quietHoursPatch } from '../settings.js';
import { clockOptions } from './notificationsModel.js';
import { OptionModel } from './optionModel.js';
import { group, switchRow } from './rows.js';

function timeRow(title, onChange) {
    const row = new Adw.ActionRow({ title });
    const model = new OptionModel();
    const dropdown = new Gtk.DropDown({ valign: Gtk.Align.CENTER });
    const state = { syncing: false };
    row.add_suffix(dropdown);
    row.activatable_widget = dropdown;
    const sync = update => {
        state.syncing = true;
        update();
        state.syncing = false;
    };
    dropdown.connect('notify::selected', () => {
        const value = model.valueAt(dropdown.selected);
        if (!state.syncing && value !== null) onChange(value);
    });
    const set = (options, value) => {
        if (model.setOptions(options)) sync(() => (dropdown.model = Gtk.StringList.new(model.labels)));
        const index = model.indexOf(value);
        if (dropdown.selected !== index) sync(() => (dropdown.selected = index));
    };
    return { row, set };
}

export class QuietHoursRows {
    constructor(client) {
        this._settings = null;
        this._clock = new DesktopClock(() => this._settings && this.update(this._settings));
        const patch = key => value => client.updateSettings(quietHoursPatch({ [key]: value }));
        this._enabled = switchRow({
            title: _('Quiet hours'),
            subtitle: _('Hold notifications while you are away'),
            onChange: patch('enabled'),
        });
        this._from = timeRow(_('From'), patch('from'));
        this._to = timeRow(_('To'), patch('to'));
        this._critical = switchRow({
            title: _('Still show critical alerts'),
            subtitle: _('Will run out and Almost out come through'),
            onChange: patch('allowCritical'),
        });
        this.group = group(
            _('Quiet Hours'),
            [this._enabled.row, this._from.row, this._to.row, this._critical.row],
            _('Held notifications arrive together when quiet hours end.')
        );
    }

    update(settings) {
        this._settings = settings;
        const hours = settings.notifications.quietHours;
        const hour12 = usesHour12(settings.display.timeFormat, this._clock.format);
        this._enabled.set(hours.enabled);
        this._from.set(clockOptions(hours.from, hour12), hours.from);
        this._to.set(clockOptions(hours.to, hour12), hours.to);
        this._critical.set(hours.allowCritical);
        for (const row of [this._from.row, this._to.row, this._critical.row]) row.sensitive = hours.enabled;
    }

    destroy() {
        this._clock.destroy();
    }
}
