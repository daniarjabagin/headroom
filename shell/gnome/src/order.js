export function moveItem(items, from, to) {
    if (from === to || from < 0 || from >= items.length) return [...items];
    const moved = [...items];
    const [item] = moved.splice(from, 1);
    moved.splice(Math.min(Math.max(to, 0), moved.length), 0, item);
    return moved;
}

export function mergeOrder(allIds, visibleOrder) {
    const visible = new Set(visibleOrder);
    const queue = [...visibleOrder];
    return allIds.map(id => (visible.has(id) ? queue.shift() : id));
}
