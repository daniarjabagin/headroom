import { PANEL_BOXES } from './displaySettings.js';
import { boxName, clampIndex, DEFAULT_POSITION, positionKey } from './panelGeometry.js';

const BOX_FIELDS = { left: '_leftBox', center: '_centerBox', right: '_rightBox' };

export class PanelPlacement {
    constructor(panel, role, button) {
        this._panel = panel;
        this._role = role;
        this._button = button;
        this._applied = positionKey(DEFAULT_POSITION);
    }

    attach() {
        this._panel.addToStatusArea(this._role, this._button, DEFAULT_POSITION.index, DEFAULT_POSITION.box);
    }

    get movable() {
        return PANEL_BOXES.every(name => this._boxActor(name) !== null);
    }

    boxes() {
        if (!this.movable) return [];
        return PANEL_BOXES.map(name => ({ name, actor: this._boxActor(name) }));
    }

    siblings(boxActor) {
        const container = this._button.container;
        return boxActor.get_children().filter(child => child !== container);
    }

    place(position) {
        const key = positionKey(position);
        if (key === this._applied) return;
        if (this.move(position)) this._applied = key;
    }

    move(position) {
        const box = this._boxActor(boxName(position.box));
        const container = this._button.container;
        if (box === null || !container) return false;
        const index = clampIndex(position.index, this.siblings(box).length);
        if (this.isCurrent({ box: boxName(position.box), index })) return true;
        this._button.menu?.close();
        container.get_parent()?.remove_child(container);
        box.insert_child_at_index(container, index);
        return true;
    }

    isCurrent(position) {
        const box = this._boxActor(boxName(position.box));
        const container = this._button.container;
        return (
            box !== null && container?.get_parent() === box && box.get_children().indexOf(container) === position.index
        );
    }

    remember(position) {
        this._applied = positionKey(position);
    }

    _boxActor(name) {
        const actor = this._panel[BOX_FIELDS[name]];
        return actor && typeof actor.insert_child_at_index === 'function' ? actor : null;
    }
}
