import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import { _ } from '../i18n.js';

const LETTER_ROWS = [13, 12, 11];
const BOTTOM_ROW = ['key', 'key', 'accent', 'space', 'key', 'key', 'key'];

export const KEYCAPS_CSS = `
.headroom-keyboard { padding: 12px; border-radius: 12px; background: alpha(currentColor, 0.06);
    border: 1px solid alpha(currentColor, 0.1); }
.headroom-key { min-width: 16px; min-height: 16px; border-radius: 4px; background: @view_bg_color;
    box-shadow: inset 0 0 0 1px alpha(currentColor, 0.12); }
.headroom-key.wide { min-width: 22px; }
.headroom-key.space { min-width: 118px; }
.headroom-key.accent { min-width: 22px; background: @accent_bg_color; box-shadow: none; }
`;

function key(kind = 'key') {
    const cssClasses = kind === 'key' ? ['headroom-key'] : ['headroom-key', kind];
    return new Gtk.Box({ css_classes: cssClasses });
}

function keyRow(kinds) {
    const row = new Gtk.Box({ spacing: 5, halign: Gtk.Align.CENTER });
    for (const kind of kinds) row.append(key(kind));
    return row;
}

export function keyboardArt() {
    const art = new Gtk.Box({
        orientation: Gtk.Orientation.VERTICAL,
        spacing: 5,
        halign: Gtk.Align.CENTER,
        css_classes: ['headroom-keyboard'],
    });
    for (const count of LETTER_ROWS) art.append(keyRow(Array.from({ length: count }, () => 'key')));
    art.append(keyRow(BOTTOM_ROW.map(kind => (kind === 'key' ? 'wide' : kind))));
    return art;
}

export function shortcutLabel(accelerator) {
    const Label = Adw.ShortcutLabel ?? Gtk.ShortcutLabel;
    return new Label({ accelerator, disabled_text: _('Disabled'), valign: Gtk.Align.CENTER });
}
