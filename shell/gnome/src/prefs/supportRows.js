import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import { group } from './rows.js';
import { REPOSITORY_URL, supportCopy } from './supportModel.js';

const LINK_ICON = 'adw-external-link-symbolic';

function openButton(label) {
    const button = new Gtk.Button({
        child: new Adw.ButtonContent({ label, icon_name: LINK_ICON }),
        valign: Gtk.Align.CENTER,
        tooltip_text: REPOSITORY_URL,
    });
    button.connect('clicked', () => new Gtk.UriLauncher({ uri: REPOSITORY_URL }).launch(button.get_root(), null, null));
    return button;
}

export function supportGroup() {
    const copy = supportCopy();
    const row = new Adw.ActionRow({ title: copy.title, subtitle: copy.subtitle, use_markup: false });
    row.add_suffix(openButton(copy.action));
    return group(copy.group, [row]);
}
