import Clutter from 'gi://Clutter';
import { _ } from '../i18n.js';
import { column, fileIcon, textButton, wrappingLabel } from '../widgets.js';

function centeredCard(children) {
    const card = column({ style_class: 'headroom-card headroom-status-card', x_expand: true });
    for (const child of children) card.add_child(child);
    return card;
}

function centered(text, styleClass) {
    return wrappingLabel(text, `${styleClass} headroom-centered`, { x_align: Clutter.ActorAlign.CENTER });
}

export function serviceView(ctx, { starting, startError }) {
    const mark = fileIcon(ctx.dir, 'headroom-symbolic.svg', 'headroom-status-mark');
    mark.x_align = Clutter.ActorAlign.CENTER;
    const children = [mark, centered(_("Headroom service isn't running"), 'headroom-status-title')];
    children.push(centered(_('Start it to see your usage limits here.'), 'headroom-status-detail'));
    const action = textButton(starting ? _('Starting…') : _('Start service'), 'headroom-primary-button', () =>
        ctx.actions.startService()
    );
    action.x_align = Clutter.ActorAlign.CENTER;
    action.reactive = !starting;
    children.push(action);
    if (startError) children.push(centered(startError, 'headroom-status-error'));
    return centeredCard(children);
}

export function errorView(ctx, message) {
    const retry = textButton(_('Try again'), 'headroom-primary-button', () => ctx.actions.refresh(''));
    retry.x_align = Clutter.ActorAlign.CENTER;
    return centeredCard([
        centered(_("Couldn't read Headroom's state"), 'headroom-status-title'),
        centered(message, 'headroom-status-detail'),
        retry,
    ]);
}

export function emptyView(ctx) {
    const retry = textButton(_('Check again'), 'headroom-small-button', () => ctx.actions.refresh(''));
    retry.x_align = Clutter.ActorAlign.CENTER;
    return centeredCard([
        centered(_('No AI coding tools found.'), 'headroom-status-detail'),
        centered(_('Sign in with a supported CLI to see your limits here.'), 'headroom-status-detail'),
        retry,
    ]);
}
