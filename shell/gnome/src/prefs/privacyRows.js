import Adw from 'gi://Adw';
import { _ } from '../i18n.js';
import { displayPatch, shortcutPatch, statusPagesPatch, supports06 } from '../settings.js';
import { shortcutLabel } from './keycaps.js';
import { group, switchRow } from './rows.js';
import { ShortcutDialog } from './shortcutDialog.js';
import { UpdateRows } from './updateRows.js';

export class ShortcutRow {
    constructor(client, subtitle) {
        this._client = client;
        this._accelerator = '';
        this.row = new Adw.ActionRow({ title: _('Open Headroom'), subtitle, activatable: true });
        this._label = shortcutLabel('');
        this.row.add_suffix(this._label);
        this.row.connect('activated', () => this._openDialog());
    }

    update(settings) {
        this._accelerator = settings.shortcuts.open;
        this._label.accelerator = this._accelerator;
    }

    _openDialog() {
        new ShortcutDialog({
            accelerator: this._accelerator,
            onSet: accelerator => this._client.updateSettings(shortcutPatch(accelerator)),
        }).present(this.row.get_root());
    }
}

export class PrivacyRows {
    constructor(client, runner) {
        this.updates = new UpdateRows(client, runner);
        this._hide = switchRow({
            title: _('Hide numbers while screen sharing'),
            subtitle: _('Shows the Headroom symbol instead of figures'),
            onChange: value => client.updateSettings(displayPatch({ hideOnScreenShare: value })),
        });
        this._statusPages = switchRow({
            title: _('Status pages'),
            subtitle: _('Show provider incidents from public status pages'),
            onChange: value => client.updateSettings(statusPagesPatch(value)),
        });
        this._shortcut = new ShortcutRow(client, _('Opens the popup from any app'));
        this._keyboard = group(_('Keyboard'), [this._shortcut.row]);
        this.groups = [
            group(_('Privacy'), [
                this._hide.row,
                this.updates.check.row,
                this.updates.status,
                this.updates.release,
                this._statusPages.row,
            ]),
            this._keyboard,
        ];
    }

    update(settings, state) {
        const supported = supports06(state);
        this._hide.set(settings.display.hideOnScreenShare);
        this._statusPages.set(settings.statusPages.enabled);
        this._shortcut.update(settings);
        this.updates.update(settings, state);
        this._hide.row.visible = supported;
        this._statusPages.row.visible = supported;
        this._keyboard.visible = supported;
    }
}
