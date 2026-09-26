import Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import St from 'gi://St';
import { lacksSubscription } from '../accountStatus.js';
import { agoText } from '../format.js';
import { _, fill } from '../i18n.js';
import { animate, HOVER_MS } from '../motion.js';
import { providerIncident } from '../providerStatus.js';
import { accountTitle } from '../providers.js';
import { label, providerIcon, row, themeIcon } from '../widgets.js';
import { busyIndicator } from './busy.js';
import { QuickLinks } from './quickLinks.js';
import { headerStatusKind, incidentTip } from './headerMarks.js';
import { statusIcon } from './statusTexts.js';

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
        this.actor = new St.Widget({
            style_class: 'headroom-section-header',
            layout_manager: new Clutter.BinLayout(),
            x_expand: true,
            reactive: true,
            track_hover: true,
        });
        this.actor.add_child(this._lead(account, showName));
        this.actor.add_child(this._trail(account));
        this.actor.connect('notify::hover', () => this._syncHover());
        this.update(account);
    }

    _lead(account, showName) {
        const lead = row({ style_class: 'headroom-header-lead', x_expand: true });
        lead.add_child(providerIcon(this._ctx.dir, account.provider, 'headroom-provider-icon'));
        const title = label(accountTitle(account, showName), 'headroom-title', { y_align: Clutter.ActorAlign.END });
        title.clutter_text.ellipsize = Pango.EllipsizeMode.END;
        lead.add_child(title);
        if (account.plan && !lacksSubscription(account))
            lead.add_child(label(account.plan, 'headroom-plan', { y_align: Clutter.ActorAlign.END }));
        this._slot = new St.Bin({ y_align: Clutter.ActorAlign.CENTER });
        this._badge = new St.Bin({ y_align: Clutter.ActorAlign.CENTER, visible: false });
        lead.add_child(this._slot);
        lead.add_child(this._badge);
        return lead;
    }

    _trail(account) {
        this._trailBox = row({
            style_class: 'headroom-header-trail',
            x_expand: true,
            x_align: Clutter.ActorAlign.END,
            y_align: Clutter.ActorAlign.CENTER,
            opacity: 0,
        });
        this._links = new QuickLinks(this._ctx, this._ctx.links(account.provider));
        this._grip = themeIcon('list-drag-handle-symbolic', 'headroom-drag-grip');
        this._trailBox.add_child(this._links.actor);
        this._trailBox.add_child(this._grip);
        return this._trailBox;
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
        const incident = this._updateIncident(account.provider);
        const kind = headerStatusKind(this._ctx, account, incident);
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
        if (key === this._incidentKey) return incident;
        this._incidentKey = key;
        this._badge.child?.destroy();
        this._badge.visible = incident !== null;
        if (!incident) return incident;
        const icon = themeIcon(statusIcon(incident), `headroom-header-status ${incident.tone}`);
        this._ctx.tooltips.attach(icon, () => {
            const current = providerIncident(this._ctx.providerStatus(), provider);
            return current ? incidentTip(this._ctx, current, this._account) : null;
        });
        this._badge.set_child(icon);
        return incident;
    }

    _syncHover() {
        const hovered = this.actor.hover;
        const links = this._links.reveal(hovered);
        this._grip.opacity = this._ctx.canReorder() ? 255 : 0;
        const shown = hovered && (links || this._ctx.canReorder());
        animate(this._ctx.motion, this._trailBox, { opacity: shown ? 255 : 0 }, { duration: HOVER_MS });
    }
}
