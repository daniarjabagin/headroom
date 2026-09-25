import { PANEL_BOXES } from './displaySettings.js';

export const DEFAULT_POSITION = Object.freeze({ box: 'right', index: 0 });

const REACH_BELOW_PANELS = 3;

export function clampIndex(index, count) {
    if (!Number.isInteger(index) || index < 0) return 0;
    return Math.min(index, Math.max(0, count));
}

export function boxName(box) {
    return PANEL_BOXES.includes(box) ? box : DEFAULT_POSITION.box;
}

export function positionKey(position) {
    return `${boxName(position.box)}:${clampIndex(position.index, Number.MAX_SAFE_INTEGER)}`;
}

function distance(box, x) {
    if (x < box.x1) return box.x1 - x;
    if (x > box.x2) return x - box.x2;
    return 0;
}

function nearestBox(boxes, x) {
    return boxes.reduce((best, box) => (distance(box, x) < distance(best, x) ? box : best));
}

function center(extent) {
    return (extent.x1 + extent.x2) / 2;
}

function insertionIndex(children, x) {
    const found = children.findIndex(child => child !== null && x < center(child));
    return found === -1 ? children.length : found;
}

function markerX(box, children, index) {
    const next = children[index];
    if (next) return next.x1;
    const shown = children.filter(child => child !== null);
    if (shown.length > 0) return shown[shown.length - 1].x2;
    return center(box);
}

export function dropTarget(boxes, x) {
    if (boxes.length === 0) return null;
    const box = nearestBox(boxes, x);
    const index = insertionIndex(box.children, x);
    return { box: box.name, index, markerX: markerX(box, box.children, index) };
}

export function withinReach(y, panelTop, panelBottom) {
    const height = Math.max(1, panelBottom - panelTop);
    return y >= panelTop - height && y <= panelBottom + REACH_BELOW_PANELS * height;
}

export function dragStarted(start, point, threshold) {
    return Math.abs(point.x - start.x) > threshold || Math.abs(point.y - start.y) > threshold;
}

export function fillWidth(fraction, width, height) {
    if (fraction <= 0) return 0;
    return Math.min(width, Math.max(height, fraction * width));
}

export function tickLeft(tick, width, tickWidth) {
    const centerX = Math.min(width - tickWidth / 2, Math.max(tickWidth / 2, tick * width));
    return centerX - tickWidth / 2;
}
