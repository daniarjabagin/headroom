import { isLightPanel, itemKey, panelLayout } from '../src/panelContent.js';
import { sameValues } from '../src/sameValues.js';
import {
    clampIndex,
    dragStarted,
    dropTarget,
    fillWidth,
    positionKey,
    tickLeft,
    withinReach,
} from '../src/panelGeometry.js';
import { PanelPlacement } from '../src/panelPlacement.js';
import { parseDisplay } from '../src/settings.js';
import { ShareHandles } from '../src/shareHandles.js';
import { grabbable, shortcutChange } from '../src/shortcutPlan.js';
import { check } from './check.js';

function item(overrides = {}) {
    return {
        accountId: 'claude:1',
        windowId: 'weekly',
        provider: 'claude',
        providerName: 'Claude',
        accountLabel: 'ada',
        windowLabel: 'Weekly',
        usedPercent: 28,
        remainingPercent: 72,
        tone: 'good',
        combined: false,
        accountCount: 1,
        valuePercent: 72,
        evenPacePercent: 41.5,
        logo: 'claude',
        ...overrides,
    };
}

function stateWith(display, items, extra = {}) {
    return {
        display: parseDisplay(display),
        panelItems: items,
        panelTone: 'warning',
        accounts: [{ id: 'claude:1', status: 'ok' }],
        offline: false,
        ...extra,
    };
}

function testPanelLayoutModes() {
    check('no state shows the mark', panelLayout(null), { mark: { tone: null }, items: [], style: 'mark' });
    const icon = stateWith({ panel_mode: 'icon' }, []);
    check('icon mode tints the mark', panelLayout(icon).mark, { tone: 'warning' });
    check('good tone stays untinted', panelLayout({ ...icon, panelTone: 'good' }).mark, { tone: null });
    check('masked icon is untinted', panelLayout(icon, { masked: true }).mark, { tone: null });
    const headline = stateWith({}, [item()]);
    check('masked headline shows only the mark', panelLayout(headline, { masked: true }).items, []);
    check('empty items fall back to the mark', panelLayout(stateWith({}, [])).mark, { tone: null });
    const invalid = stateWith({ panel_indicator: 'none', panel_label: 'none' }, [item()]);
    check('none and none shows the mark', panelLayout(invalid).mark, { tone: null });
}

function testHeadlineItem() {
    const [ring] = panelLayout(stateWith({}, [item()])).items;
    check('headline item', ring, {
        key: 'claude:1\u0000weekly',
        logo: null,
        letter: null,
        count: null,
        indicator: 'ring',
        fraction: 0.72,
        tick: null,
        value: '72%',
        tone: 'good',
        stale: false,
    });
    const [bar] = panelLayout(stateWith({ panel_indicator: 'bar' }, [item()])).items;
    check('bar tick follows the left reading', bar.tick, 1 - 0.415);
    const used = stateWith({ panel_indicator: 'bar', value_mode: 'used' }, [item({ valuePercent: 28 })]);
    check('bar tick follows the used reading', panelLayout(used).items[0].tick, 0.415);
    const noPace = stateWith({ panel_indicator: 'bar' }, [item({ evenPacePercent: null })]);
    check('no pace hides the tick', panelLayout(noPace).items[0].tick, null);
    const ringOnly = stateWith({ panel_label: 'none' }, [item()]);
    check('ring only has no value', panelLayout(ringOnly).items[0].value, null);
}

function testWindowLabels() {
    const windowed = stateWith({ panel_label: 'window' }, [item({ combined: true, accountCount: 2 })]);
    const [current] = panelLayout(windowed).items;
    check('window label shows logo and letter', [current.logo, current.letter, current.count], ['claude', 'W', '×2']);
    check('window label keeps the indicator', current.indicator, 'ring');
    check('legacy window label drops the ring', panelLayout(windowed, { legacy: true }).items[0].indicator, null);
    const several = stateWith({ panel_mode: 'several' }, [
        item(),
        item({ accountId: 'codex:2', logo: null, provider: 'codex' }),
    ]);
    const layout = panelLayout(several);
    check(
        'several logos',
        layout.items.map(entry => entry.logo),
        ['claude', 'codex']
    );
    check(
        'several letters',
        layout.items.map(entry => entry.letter),
        ['W', 'W']
    );
    check('several style', layout.style, 'several:ring:percent');
}

function testStaleAndKeys() {
    const offline = stateWith({}, [item()], { offline: true });
    check('offline items are stale', panelLayout(offline).items[0].stale, true);
    const stale = stateWith({}, [item()], { accounts: [{ id: 'claude:1', status: 'stale' }] });
    check('stale account', panelLayout(stale).items[0].stale, true);
    check('item key', itemKey({ accountId: 'a', windowId: 'b' }), 'a\u0000b');
    check(
        'same keys',
        [sameValues(['a', 'b'], ['a', 'b']), sameValues(['a'], ['b']), sameValues(['a'], ['a', 'b'])],
        [true, false, false]
    );
}

function testLightPanel() {
    check('dark foreground is a light panel', isLightPanel({ red: 30, green: 30, blue: 30 }), true);
    check('white foreground is a dark panel', isLightPanel({ red: 255, green: 255, blue: 255 }), false);
    check('mid gray', isLightPanel({ red: 200, green: 200, blue: 200 }), false);
}

function testPlacementMath() {
    check('clamp inside', clampIndex(2, 5), 2);
    check('clamp beyond', clampIndex(9, 3), 3);
    check('clamp negative', clampIndex(-1, 3), 0);
    check('clamp non-integer', clampIndex(1.5, 3), 0);
    check('clamp empty box', clampIndex(4, 0), 0);
    check('position key', positionKey({ box: 'left', index: 2 }), 'left:2');
    check('unknown box maps right', positionKey({ box: 'top', index: 1 }), 'right:1');
}

function panelBoxes() {
    return [
        { name: 'left', x1: 0, x2: 100, children: [{ x1: 0, x2: 60 }, null, { x1: 60, x2: 100 }] },
        { name: 'center', x1: 500, x2: 620, children: [{ x1: 500, x2: 620 }] },
        { name: 'right', x1: 1100, x2: 1280, children: [null, { x1: 1100, x2: 1180 }, { x1: 1180, x2: 1280 }] },
    ];
}

function testDropTargets() {
    const boxes = panelBoxes();
    check('before the first left child', dropTarget(boxes, 10), { box: 'left', index: 0, markerX: 0 });
    check('after a hidden child', dropTarget(boxes, 70), { box: 'left', index: 2, markerX: 60 });
    check('end of the left box', dropTarget(boxes, 200), { box: 'left', index: 3, markerX: 100 });
    check('after the clock', dropTarget(boxes, 590), { box: 'center', index: 1, markerX: 620 });
    check('before the clock', dropTarget(boxes, 450), { box: 'center', index: 0, markerX: 500 });
    check('right box skips hidden', dropTarget(boxes, 1105), { box: 'right', index: 1, markerX: 1100 });
    check('right box end', dropTarget(boxes, 1279), { box: 'right', index: 3, markerX: 1280 });
    const empty = [{ name: 'center', x1: 600, x2: 600, children: [] }];
    check('empty box marker', dropTarget(empty, 10), { box: 'center', index: 0, markerX: 600 });
    check('no boxes', dropTarget([], 10), null);
    check(
        'within reach',
        [withinReach(10, 0, 32), withinReach(120, 0, 32), withinReach(200, 0, 32)],
        [true, true, false]
    );
    check(
        'drag threshold',
        [dragStarted({ x: 0, y: 0 }, { x: 8, y: 0 }, 8), dragStarted({ x: 0, y: 0 }, { x: 0, y: 9 }, 8)],
        [false, true]
    );
}

function testBarGeometry() {
    check('empty bar', fillWidth(0, 26, 5), 0);
    check('tiny value keeps a dot', fillWidth(0.01, 26, 5), 5);
    check('half bar', fillWidth(0.5, 26, 5), 13);
    check('full bar', fillWidth(1, 26, 5), 26);
    check('tick centered', tickLeft(0.5, 26, 2), 12);
    check('tick clamped left', tickLeft(0, 26, 2), 0);
    check('tick clamped right', tickLeft(1, 26, 2), 24);
}

class FakeBox {
    constructor(children) {
        this.children = children;
    }

    get_children() {
        return [...this.children];
    }

    insert_child_at_index(child, index) {
        child.parent = this;
        this.children.splice(index, 0, child);
    }

    remove_child(child) {
        this.children = this.children.filter(entry => entry !== child);
        child.parent = null;
    }
}

function fakePanel() {
    const container = {
        parent: null,
        get_parent() {
            return this.parent;
        },
    };
    const button = { container, closed: 0, menu: { close: () => (button.closed += 1) } };
    const panel = {
        _leftBox: new FakeBox(['activities']),
        _centerBox: new FakeBox(['clock']),
        _rightBox: new FakeBox(['a11y', 'quick']),
        statusArea: {},
        addToStatusArea(role, indicator, index, box) {
            this.statusArea[role] = indicator;
            ({ left: this._leftBox, center: this._centerBox, right: this._rightBox })[box].insert_child_at_index(
                indicator.container,
                index
            );
        },
    };
    return { panel, button, container };
}

function testPlacement() {
    const { panel, button, container } = fakePanel();
    const placement = new PanelPlacement(panel, 'headroom', button);
    placement.attach();
    check('attached at the default', panel._rightBox.children.indexOf(container), 0);
    placement.place({ box: 'left', index: 7 });
    check('clamped into the left box', panel._leftBox.children.indexOf(container), 1);
    check('left the right box', panel._rightBox.children.includes(container), false);
    placement.place({ box: 'left', index: 7 });
    check('same position is not re-placed', button.closed, 1);
    placement.place({ box: 'center', index: 0 });
    check('before the clock', panel._centerBox.children.indexOf(container), 0);
    check('is current', placement.isCurrent({ box: 'center', index: 0 }), true);
    check('siblings exclude the indicator', placement.siblings(panel._centerBox), ['clock']);
    placement.remember({ box: 'right', index: 1 });
    placement.place({ box: 'right', index: 1 });
    check('remembered position is skipped', panel._centerBox.children.includes(container), true);
    check('movable', placement.movable, true);
    const broken = new PanelPlacement({ addToStatusArea() {} }, 'x', button);
    check(
        'private boxes missing',
        [broken.movable, broken.boxes(), broken.move({ box: 'left', index: 0 })],
        [false, [], false]
    );
}

function testShareHandles() {
    const handles = new ShareHandles();
    const first = { id: 1 };
    const second = { id: 2 };
    check('idle is not masked', handles.masked(true), false);
    check('first handle starts sharing', handles.add(first), true);
    check('second handle keeps sharing', handles.add(second), false);
    check('duplicate handle ignored', handles.add(first), false);
    check('masked while sharing', handles.masked(true), true);
    check('setting off shows numbers', handles.masked(false), false);
    check('reveal', handles.reveal(), true);
    check('revealed is not masked', handles.masked(true), false);
    check('one handle stops', handles.stop(first), false);
    check('still revealed', handles.masked(true), false);
    check('unknown handle stop', handles.stop({ id: 3 }), false);
    check('last handle stops', handles.stop(second), true);
    check('reveal resets after the share', handles.revealed, false);
    handles.add(first);
    check('new share masks again', handles.masked(true), true);
    check('reveal without share', new ShareHandles().reveal(), false);
}

function testShortcutPlan() {
    check('valid accelerator', grabbable('<Super>u'), '<Super>u');
    check('invalid accelerator', grabbable('<Super>'), '');
    check('non-string', grabbable(null), '');
    check('unchanged', shortcutChange('<Super>u', '<Super>u'), null);
    check('first grab', shortcutChange('', '<Control><Alt>h'), { release: false, grab: '<Control><Alt>h' });
    check('change', shortcutChange('<Super>u', '<Super>h'), { release: true, grab: '<Super>h' });
    check('disable', shortcutChange('<Super>u', ''), { release: true, grab: '' });
    check('invalid disables', shortcutChange('<Super>u', 'Super+U'), { release: true, grab: '' });
    check('empty stays empty', shortcutChange('', 'bad key'), null);
}

export function testPanel() {
    testPanelLayoutModes();
    testHeadlineItem();
    testWindowLabels();
    testStaleAndKeys();
    testLightPanel();
    testPlacementMath();
    testDropTargets();
    testBarGeometry();
    testPlacement();
    testShareHandles();
    testShortcutPlan();
}
