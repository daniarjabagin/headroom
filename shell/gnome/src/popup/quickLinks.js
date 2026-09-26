import Clutter from 'gi://Clutter';
import { button, row } from '../widgets.js';
import { shownLinks } from './cardMenu.js';
import { popupIcon } from './icons.js';

const ORDER = ['status', 'usage', 'dashboard'];

function linkButton(ctx, link) {
    const actor = button(popupIcon(ctx.dir, link.icon, 'headroom-quick-link-icon'), 'headroom-quick-link', () =>
        ctx.actions.openUrl(link.url)
    );
    actor.accessible_name = link.title;
    ctx.tooltips.attach(actor, () => `${link.title} · ${link.host}`);
    return actor;
}

export class QuickLinks {
    constructor(ctx, links) {
        const shown = shownLinks(links).sort((a, b) => ORDER.indexOf(a.key) - ORDER.indexOf(b.key));
        this.actor = row({ style_class: 'headroom-quick-links', y_align: Clutter.ActorAlign.CENTER });
        for (const link of shown) this.actor.add_child(linkButton(ctx, link));
        this.actor.visible = shown.length > 0;
        this.reveal(false);
    }

    reveal(shown) {
        for (const child of this.actor.get_children()) {
            child.reactive = shown;
            child.can_focus = shown;
        }
        return this.actor.visible;
    }
}
