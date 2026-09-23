import Clutter from 'gi://Clutter';
import Graphene from 'gi://Graphene';
import St from 'gi://St';
import { animate } from '../motion.js';
import { button, themeIcon } from '../widgets.js';

const OPEN_ANGLE = 180;

export class Expander {
    constructor(ctx, child, { expanded, onToggled }) {
        this._ctx = ctx;
        this._onToggled = onToggled;
        this._expanded = expanded;
        this._caret = themeIcon('pan-down-symbolic', 'headroom-caret-icon');
        this._caret.pivot_point = new Graphene.Point({ x: 0.5, y: 0.5 });
        this.toggle = button(this._caret, 'headroom-caret', () => this._toggle());
        this.toggle.x_expand = true;
        this.content = new St.Bin({ child, x_expand: true, clip_to_allocation: true });
        this.content.x_align = Clutter.ActorAlign.FILL;
        this.content.visible = expanded;
        this._caret.rotation_angle_z = expanded ? OPEN_ANGLE : 0;
    }

    _toggle() {
        this._expanded = !this._expanded;
        this._onToggled(this._expanded);
        const motion = this._ctx.motion;
        animate(motion, this._caret, { rotation_angle_z: this._expanded ? OPEN_ANGLE : 0 });
        if (this._expanded) this._open(motion);
        else this._close(motion);
    }

    _currentHeight() {
        this.content.remove_transition('height');
        return this.content.visible ? this.content.height : 0;
    }

    _naturalHeight() {
        this.content.height = -1;
        const [, natural] = this.content.get_preferred_height(this.content.get_parent().width);
        return natural;
    }

    _open(motion) {
        const from = this._currentHeight();
        this.content.show();
        const target = this._naturalHeight();
        this.content.height = from;
        animate(motion, this.content, { height: target }, { onComplete: () => (this.content.height = -1) });
    }

    _close(motion) {
        this.content.height = this._currentHeight();
        animate(
            motion,
            this.content,
            { height: 0 },
            {
                onComplete: () => {
                    this.content.hide();
                    this.content.height = -1;
                },
            }
        );
    }
}
