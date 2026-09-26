import Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import St from 'gi://St';
import { _, fill, n_ } from '../i18n.js';
import { button, label, providerIcon, row, themeIcon } from '../widgets.js';
import { attentionText } from './collapsedAttention.js';

const MAX_ICONS = 3;

function cardProvider(card) {
    return card.kind === 'combined'
        ? { provider: card.group.provider, name: card.group.providerName }
        : { provider: card.account.provider, name: card.account.providerName };
}

export function collapsedNames(cards) {
    return [...new Set(cards.map(card => cardProvider(card).name))].join(', ');
}

export function moreTitle(count) {
    return fill(n_('{count} more', '{count} more', count), { count });
}

function collapsedAccessibleName(cards, attention) {
    return [`${moreTitle(cards.length)} · ${collapsedNames(cards)}`, attentionText(attention)]
        .filter(part => part !== null)
        .join('. ');
}

function icons(ctx, cards) {
    const box = row({ style_class: 'headroom-more-icons', y_align: Clutter.ActorAlign.CENTER });
    const providers = [...new Set(cards.map(card => cardProvider(card).provider))].slice(0, MAX_ICONS);
    for (const provider of providers) box.add_child(providerIcon(ctx.dir, provider, 'headroom-more-icon'));
    return box;
}

function attentionMark(ctx, attention) {
    const mark =
        attention.kind === 'notice'
            ? themeIcon('dialog-warning-symbolic', `headroom-more-attention ${attention.tone}`)
            : new St.Widget({
                  style_class: `headroom-more-dot ${attention.tone}`,
                  y_align: Clutter.ActorAlign.CENTER,
              });
    ctx.tooltips.attach(mark, () => attentionText(attention));
    return mark;
}

function collapsedView(ctx, cards, attention, onToggle) {
    const content = row({ style_class: 'headroom-more-box', x_expand: true });
    content.add_child(icons(ctx, cards));
    content.add_child(label(moreTitle(cards.length), 'headroom-more-title'));
    const names = label(`· ${collapsedNames(cards)}`, 'headroom-more-names', { x_expand: true });
    names.clutter_text.ellipsize = Pango.EllipsizeMode.END;
    content.add_child(names);
    if (attention.kind !== 'none') content.add_child(attentionMark(ctx, attention));
    content.add_child(themeIcon('pan-end-symbolic', 'headroom-more-caret'));
    const actor = button(content, 'headroom-more-row', onToggle);
    actor.x_expand = true;
    actor.accessible_name = collapsedAccessibleName(cards, attention);
    return actor;
}

function expandedView(onToggle) {
    const actor = row({ style_class: 'headroom-more-head', x_expand: true });
    actor.add_child(label(_('Not pinned'), 'headroom-more-head-title'));
    actor.add_child(
        new St.Widget({ style_class: 'headroom-more-rule', x_expand: true, y_align: Clutter.ActorAlign.CENTER })
    );
    const less = row({ style_class: 'headroom-more-less-box' });
    less.add_child(label(_('Show less'), 'headroom-more-less'));
    less.add_child(themeIcon('pan-up-symbolic', 'headroom-more-less-icon'));
    actor.add_child(button(less, 'headroom-more-less-button', onToggle));
    return actor;
}

export function collapsedRow(ctx, cards, attention, expanded, onToggle) {
    return expanded ? expandedView(onToggle) : collapsedView(ctx, cards, attention, onToggle);
}
