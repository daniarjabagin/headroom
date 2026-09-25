import Clutter from 'gi://Clutter';
import St from 'gi://St';
import { _ } from '../i18n.js';
import { providerIncident } from '../providerStatus.js';
import { button, column, label, row, spacer, themeIcon, wrappingLabel } from '../widgets.js';
import { popupIcon } from './icons.js';
import {
    statusAge,
    statusDetail,
    statusIcon,
    statusKind,
    statusStarted,
    statusTitle,
    statusTone,
} from './statusTexts.js';

function statusLink(ctx, urlOf) {
    const content = row({ style_class: 'headroom-status-link-box' });
    content.add_child(label(_('Status page'), 'headroom-status-link-text'));
    content.add_child(popupIcon(ctx.dir, 'external', 'headroom-status-link-icon'));
    const actor = button(content, 'headroom-status-link', () => {
        const url = urlOf();
        if (url) ctx.actions.openUrl(url);
    });
    return actor;
}

function iconTile(status) {
    const tile = new St.Bin({
        style_class: `headroom-notice-tile ${statusTone(status)}`,
        y_align: Clutter.ActorAlign.START,
    });
    tile.set_child(themeIcon(statusIcon(status), 'headroom-notice-icon'));
    return tile;
}

class FullNotice {
    constructor(ctx, status, urlOf) {
        this.actor = row({ style_class: `headroom-notice ${statusTone(status)}`, x_expand: true });
        const texts = column({ style_class: 'headroom-notice-texts', x_expand: true });
        this._title = wrappingLabel('', 'headroom-notice-title');
        this._detail = wrappingLabel('', 'headroom-notice-detail');
        const meta = row({ style_class: 'headroom-notice-meta' });
        this._started = label('', 'headroom-notice-detail');
        meta.add_child(this._started);
        meta.add_child(spacer());
        meta.add_child(statusLink(ctx, urlOf));
        for (const actor of [this._title, this._detail, meta]) texts.add_child(actor);
        this.actor.add_child(iconTile(status));
        this.actor.add_child(texts);
    }

    update(status, now) {
        this._title.text = statusTitle(status);
        const detail = statusDetail(status);
        this._detail.text = detail ?? '';
        this._detail.visible = detail !== null;
        this._started.text = statusStarted(status, now) ?? '';
    }
}

class CompactNotice {
    constructor(ctx, status, urlOf) {
        this.actor = row({ style_class: 'headroom-status-line', x_expand: true });
        this.actor.add_child(
            new St.Widget({ style_class: `headroom-status-dot ${status.tone}`, y_align: Clutter.ActorAlign.CENTER })
        );
        this._kind = label('', 'headroom-status-kind');
        this._age = label('', 'headroom-status-age');
        for (const actor of [this._kind, this._age, spacer(), statusLink(ctx, urlOf)]) this.actor.add_child(actor);
        ctx.tooltips.attach(this._kind, () => (this._status ? statusTitle(this._status) : null));
    }

    update(status, now) {
        this._status = status;
        this._kind.text = statusKind(status);
        this._age.text = statusAge(status, now) ?? '';
    }
}

export class StatusNotice {
    constructor(ctx, provider) {
        this._ctx = ctx;
        this._provider = provider;
        const status = this._status();
        const urlOf = () => this._status()?.url ?? null;
        const Kind = ctx.display.density === 'compact' ? CompactNotice : FullNotice;
        this._view = new Kind(ctx, status, urlOf);
        this.actor = this._view.actor;
        this.update();
    }

    _status() {
        return providerIncident(this._ctx.providerStatus(), this._provider);
    }

    update() {
        const status = this._status();
        if (status) this._view.update(status, this._ctx.now());
    }
}
