import Clutter from 'gi://Clutter';
import { _ } from '../i18n.js';
import { row, textButton, themeIcon, wrappingLabel } from '../widgets.js';

export class MaskBanner {
    constructor(onReveal) {
        this.actor = row({ style_class: 'headroom-mask-banner', x_expand: true, visible: false });
        const icon = themeIcon('view-conceal-symbolic', 'headroom-mask-icon');
        icon.y_align = Clutter.ActorAlign.CENTER;
        this._text = wrappingLabel('', 'headroom-mask-text');
        this._button = textButton('', 'headroom-small-button', () => onReveal());
        this._button.y_align = Clutter.ActorAlign.CENTER;
        for (const actor of [icon, this._text, this._button]) this.actor.add_child(actor);
        this.relabel();
    }

    relabel() {
        this._text.text = _('Numbers hidden while your screen is shared');
        this._button.label = _('Show anyway');
    }

    show(visible) {
        this.actor.visible = visible;
    }
}
