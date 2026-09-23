import Clutter from 'gi://Clutter';
import { column, fileIcon, label, textButton, wrappingLabel } from '../widgets.js';

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
    const children = [mark, centered("Headroom service isn't running", 'headroom-status-title')];
    children.push(centered('Start it to see your usage limits here.', 'headroom-status-detail'));
    const action = textButton(starting ? 'Starting…' : 'Start service', 'headroom-primary-button', () =>
        ctx.actions.startService()
    );
    action.x_align = Clutter.ActorAlign.CENTER;
    action.reactive = !starting;
    children.push(action);
    if (startError) children.push(centered(startError, 'headroom-status-error'));
    return centeredCard(children);
}

export function loadingView() {
    return centeredCard([label('Loading…', 'headroom-status-detail', { x_align: Clutter.ActorAlign.CENTER })]);
}

export function errorView(ctx, message) {
    const retry = textButton('Try again', 'headroom-primary-button', () => ctx.actions.refresh(''));
    retry.x_align = Clutter.ActorAlign.CENTER;
    return centeredCard([
        centered("Couldn't read Headroom's state", 'headroom-status-title'),
        centered(message, 'headroom-status-detail'),
        retry,
    ]);
}

export function emptyView(ctx) {
    const retry = textButton('Check again', 'headroom-small-button', () => ctx.actions.refresh(''));
    retry.x_align = Clutter.ActorAlign.CENTER;
    return centeredCard([centered('No AI coding tools found.', 'headroom-status-detail'), retry]);
}
