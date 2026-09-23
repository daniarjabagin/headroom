import Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import St from 'gi://St';
import * as Animation from 'resource:///org/gnome/shell/ui/animation.js';
import { lacksSubscription } from '../accountStatus.js';
import { agoText } from '../format.js';
import { _, fill } from '../i18n.js';
import { animate, FAST_MS } from '../motion.js';
import { accountTitle } from '../providers.js';
import { label, providerIcon, row, spacer, themeIcon } from '../widgets.js';

export function failedOffline(ctx, account) {
    return ctx.offline && account.error?.kind === 'network';
}

function statusKind(ctx, account) {
    if (account.status === 'refreshing') return 'refreshing';
    if (account.status === 'stale' || (account.status === 'error' && failedOffline(ctx, account))) return 'outdated';
    if (account.status === 'error') return 'error';
    return null;
}

function busyIndicator(motion, size, styleClass) {
    if (!motion.enabled) return themeIcon('view-refresh-symbolic', `${styleClass} static`);
    const spinner = new Animation.Spinner(size, { animate: true });
    spinner.add_style_class_name(styleClass);
    spinner.y_align = Clutter.ActorAlign.CENTER;
    spinner.play();
    return spinner;
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

export class AccountHeader {
    constructor(ctx, account, showName) {
        this._ctx = ctx;
        this._kind = undefined;
        this.actor = row({ style_class: 'headroom-section-header', reactive: true, track_hover: true });
        this.actor.add_child(providerIcon(ctx.dir, account.provider, 'headroom-provider-icon'));
        const title = label(accountTitle(account, showName), 'headroom-title', { y_align: Clutter.ActorAlign.END });
        title.clutter_text.ellipsize = Pango.EllipsizeMode.END;
        this.actor.add_child(title);
        if (account.plan && !lacksSubscription(account))
            this.actor.add_child(label(account.plan, 'headroom-plan', { y_align: Clutter.ActorAlign.END }));
        this._slot = new St.Bin({ y_align: Clutter.ActorAlign.CENTER });
        this.actor.add_child(this._slot);
        this.actor.add_child(spacer());
        this._grip = themeIcon('list-drag-handle-symbolic', 'headroom-drag-grip');
        this._grip.opacity = 0;
        this.actor.add_child(this._grip);
        this.actor.connect('notify::hover', () => this._syncGrip());
        this.update(account);
    }

    update(account) {
        this._account = account;
        const kind = statusKind(this._ctx, account);
        if (kind === this._kind) return;
        this._kind = kind;
        this._slot.child?.destroy();
        const child = statusActor(this._ctx, kind, () => this._account);
        this._slot.set_child(child);
        this._slot.visible = child !== null;
    }

    _syncGrip() {
        const shown = this.actor.hover && this._ctx.canReorder();
        animate(this._ctx.motion, this._grip, { opacity: shown ? 255 : 0 }, { duration: FAST_MS });
    }
}
