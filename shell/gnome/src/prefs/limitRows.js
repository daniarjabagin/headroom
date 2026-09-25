import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import { _ } from '../i18n.js';
import { panelLimitsPatch } from '../settings.js';
import { canChoose, limitChoices, limitsAfter, limitsSubtitle } from './panelModel.js';
import { providerImage } from './widgets.js';

function choiceRow(dir, choice, onToggle) {
    const check = new Gtk.CheckButton({ valign: Gtk.Align.CENTER });
    const row = new Adw.ActionRow({ title: choice.title, use_markup: false, activatable_widget: check });
    row.add_prefix(providerImage(dir, choice.provider, 24));
    row.add_prefix(check);
    check.connect('toggled', () => onToggle(check.active));
    return { row, check };
}

export class LimitRows {
    constructor(client, dir) {
        this._client = client;
        this._dir = dir;
        this._syncing = false;
        this._entries = [];
        this._shape = '';
        this.row = new Adw.ExpanderRow({ title: _('Limits in the panel') });
    }

    update(settings, state) {
        const chosen = settings.display.panelLimits;
        const choices = limitChoices(state.accounts, chosen);
        const shape = JSON.stringify(choices.map(choice => [choice.key, choice.title, choice.provider]));
        if (shape !== this._shape) this._rebuild(choices, shape);
        this._syncing = true;
        this.row.subtitle = limitsSubtitle(chosen.length);
        choices.forEach((choice, index) => {
            const entry = this._entries[index];
            entry.row.subtitle = choice.subtitle;
            entry.check.active = choice.checked;
            entry.row.sensitive = canChoose(chosen.length, choice.checked);
        });
        this._syncing = false;
    }

    _rebuild(choices, shape) {
        for (const entry of this._entries) this.row.remove(entry.row);
        this._entries = choices.map(choice => choiceRow(this._dir, choice, checked => this._toggle(choice, checked)));
        for (const entry of this._entries) this.row.add_row(entry.row);
        this._shape = shape;
    }

    _toggle(choice, checked) {
        const display = this._client.settings?.display;
        if (this._syncing || !display) return;
        this._client.updateSettings(panelLimitsPatch(limitsAfter(display.panelLimits, choice, checked)));
    }
}
