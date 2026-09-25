import Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import St from 'gi://St';
import { lacksSubscription } from '../accountStatus.js';
import { agoText } from '../format.js';
import { _, fill } from '../i18n.js';
import { animate, FAST_MS } from '../motion.js';
import { providerIncident } from '../providerStatus.js';
import { accountTitle } from '../providers.js';
import { label, providerIcon, row, spacer, themeIcon } from '../widgets.js';
import { busyIndicator } from './busy.js';
import { QuickLinks } from './quickLinks.js';
import { statusIcon, statusTitle } from './statusTexts.js';

export function failedOffline(ctx, account) {
    return ctx.offline && account.error?.kind === 'network';
}

function statusKind(ctx, account) {
    if (account.status === 'refreshing') return 'refreshing';
    if (account.status === 'stale' || (account.status === 'error' && failedOffline(ctx, account))) return 'outdated';
    if (account.status === 'error') return 'error';
    return null;
}

function outdatedTag(ctx, accountOf) {
    const tag = label(_('Outdated'), 'headroom-stale-tag');
    ctx.tooltips.attach(tag, () => {
        const updatedAt = accountOf().updatedAt;
        return updatedAt && fill(_('Last updated {ago}'), { ago: agoText(updatedAt, ctx.now()) });
    });
    return tag;
}

function statusActor(ctx, kind, accountOf) {
    if (kind === 'refreshing') return busyIndicator(ctx.motion, 12, 'headroom-header-spinner');
    if (kind === 'outdated') return outdatedTag(ctx, accountOf);
    if (kind === 'error') {
        const icon = themeIcon('dialog-warning-symbolic', 'headroom-header-warning');
        ctx.tooltips.attach(icon, () => accountOf().error?.message ?? _('Refresh failed'));
        return icon;
    }
    return null;
}

function incidentKey(incident) {
    return incident ? `${incident.tone}:${incident.indicator}` : null;
}

export class AccountHeader {
    constructor(ctx, account, showName) {
        this._ctx = ctx;
        this._kind = undefined;
        this._incidentKey = undefined;
        this.actor = row({ style_class: 'headroom-section-header', reactive: true, track_hover: true });
        this.actor.add_child(providerIcon(ctx.dir, account.provider, 'headroom-provider-icon'));
        const title = label(accountTitle(account, showName), 'headroom-title', { y_align: Clutter.ActorAlign.END });
        title.clutter_text.ellipsize = Pango.EllipsizeMode.END;
        this.actor.add_child(title);
        if (account.plan && !lacksSubscription(account))
            this.actor.add_child(label(account.plan, 'headroom-plan', { y_align: Clutter.ActorAlign.END }));
        this._slot = new St.Bin({ y_align: Clutter.ActorAlign.CENTER });
        this._badge = new St.Bin({ y_align: Clutter.ActorAlign.CENTER, visible: false });
        this.actor.add_child(this._slot);
        this.actor.add_child(this._badge);
        this.actor.add_child(spacer());
        this._links = new QuickLinks(ctx, ctx.links(account.provider));
        this.actor.add_child(this._links.actor);
        this._grip = themeIcon('list-drag-handle-symbolic', 'headroom-drag-grip');
        this._grip.opacity = 0;
        this.actor.add_child(this._grip);
        this.actor.connect('notify::hover', () => this._syncHover());
        this.update(account);
    }

    onSecondaryClick(handler) {
        this.actor.connect('button-press-event', (_actor, event) => {
            if (event.get_button() !== Clutter.BUTTON_SECONDARY) return Clutter.EVENT_PROPAGATE;
            handler(event.get_coords());
            return Clutter.EVENT_STOP;
        });
    }

    update(account) {
        this._account = account;
        this._updateIncident(account.provider);
        const kind = statusKind(this._ctx, account);
        if (kind === this._kind) return;
        this._kind = kind;
        this._slot.child?.destroy();
        const child = statusActor(this._ctx, kind, () => this._account);
        this._slot.set_child(child);
        this._slot.visible = child !== null;
    }

    _updateIncident(provider) {
        const incident = providerIncident(this._ctx.providerStatus(), provider);
        const key = incidentKey(incident);
        if (key === this._incidentKey) return;
        this._incidentKey = key;
        this._badge.child?.destroy();
        this._badge.visible = incident !== null;
        if (!incident) return;
        const icon = themeIcon(statusIcon(incident), `headroom-header-status ${incident.tone}`);
        this._ctx.tooltips.attach(icon, () => {
            const current = providerIncident(this._ctx.providerStatus(), provider);
            return current ? statusTitle(current) : null;
        });
        this._badge.set_child(icon);
    }

    _syncHover() {
        const hovered = this.actor.hover;
        this._links.reveal(hovered);
        const shown = hovered && this._ctx.canReorder();
        animate(this._ctx.motion, this._grip, { opacity: shown ? 255 : 0 }, { duration: FAST_MS });
    }
}
