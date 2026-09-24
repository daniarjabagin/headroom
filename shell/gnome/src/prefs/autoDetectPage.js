import Gtk from 'gi://Gtk';
import { _ } from '../i18n.js';
import { flowBody, navigationPage, stack } from './flowPage.js';
import { pillButton, wrapLabel } from './widgets.js';

export class AutoDetectPage {
    constructor({ dir, provider, method, onRestore }) {
        this._provider = provider.id;
        this._onRestore = onRestore;
        const { box, description } = flowBody(dir, provider.id, provider.displayName);
        description.label = method.reason ?? _('Headroom finds this account on its own.');
        this._status = wrapLabel('', ['dim-label', 'caption']);
        this._status.justify = Gtk.Justification.CENTER;
        this._status.visible = false;
        this._button = pillButton(_('Rescan'), true, () => this._rescan());
        const note = wrapLabel(_('Already set up, or removed it earlier? Rescan to find it again.'), ['caption']);
        note.justify = Gtk.Justification.CENTER;
        box.append(stack([note, this._button, this._status]));
        this.page = navigationPage(provider.displayName, box);
    }

    cancel() {}

    async _rescan() {
        this._button.sensitive = false;
        this._status.visible = true;
        this._status.label = _('Looking for accounts…');
        const finished = await this._onRestore(this._provider);
        this._button.sensitive = true;
        this._status.label = finished
            ? _('Scan finished. Accounts that were found appear in the list.')
            : _("Couldn't reach the Headroom service.");
    }
}
