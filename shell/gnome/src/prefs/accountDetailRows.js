import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import { windowLabel } from '../format.js';
import { _ } from '../i18n.js';
import { accountName } from '../providers.js';
import { shownLinks } from '../popup/cardMenu.js';

function removeSubtitle(account) {
    if (account.owner === 'headroom') return _('Deletes the sign-in Headroom created for this account');
    return _('Stops showing this account. Its CLI stays signed in.');
}

function iconButton(iconName, tooltip, onClick) {
    const button = new Gtk.Button({ icon_name: iconName, tooltip_text: tooltip, valign: Gtk.Align.CENTER });
    button.connect('clicked', () => onClick());
    return button;
}

function suffixButton(label, cssClass, onClick) {
    const button = new Gtk.Button({ label, valign: Gtk.Align.CENTER });
    if (cssClass) button.add_css_class(cssClass);
    button.connect('clicked', () => onClick());
    return button;
}

export function windowRow(window, onToggle) {
    const row = new Adw.SwitchRow({ title: windowLabel(window.id, window.label), use_markup: false });
    row.connect('notify::active', () => onToggle(row.active));
    return row;
}

export function signInGroup(account, onSignIn) {
    const row = new Adw.ActionRow({
        title: accountName(account),
        subtitle: _('The sign-in for this account has expired'),
        use_markup: false,
    });
    row.add_prefix(new Gtk.Image({ icon_name: 'dialog-warning-symbolic', css_classes: ['warning'] }));
    row.add_suffix(suffixButton(_('Sign in again…'), 'suggested-action', onSignIn));
    const group = new Adw.PreferencesGroup();
    group.add(row);
    return group;
}

export function linksGroup(links, onOpen) {
    const entries = shownLinks(links);
    if (entries.length === 0) return null;
    const group = new Adw.PreferencesGroup({ title: _('Links') });
    for (const link of entries) {
        const row = new Adw.ActionRow({ title: link.title, subtitle: link.host, activatable: true });
        row.add_suffix(new Gtk.Image({ icon_name: 'adw-external-link-symbolic' }));
        row.connect('activated', () => onOpen(link.url));
        group.add(row);
    }
    return group;
}

export function positionRow(onMove) {
    const row = new Adw.ActionRow({ title: _('Position') });
    const box = new Gtk.Box({ css_classes: ['linked'], valign: Gtk.Align.CENTER });
    const up = iconButton('go-up-symbolic', _('Move up'), () => onMove(-1));
    const down = iconButton('go-down-symbolic', _('Move down'), () => onMove(1));
    box.append(up);
    box.append(down);
    row.add_suffix(box);
    return { row, up, down };
}

export function removeRow(account, onRemove) {
    const row = new Adw.ActionRow({ title: _('Remove from Headroom'), subtitle: removeSubtitle(account) });
    row.add_suffix(suffixButton(_('Remove…'), 'destructive-action', onRemove));
    return row;
}
