.pragma library

function moveItem(items, from, to) {
    if (from === to || from < 0 || from >= items.length)
        return items.slice();
    const moved = items.slice();
    const [item] = moved.splice(from, 1);
    moved.splice(Math.min(Math.max(to, 0), moved.length), 0, item);
    return moved;
}

function mergeOrder(allIds, visibleOrder) {
    const visible = new Set(visibleOrder);
    const queue = visibleOrder.slice();
    return allIds.map(id => visible.has(id) ? queue.shift() : id);
}

function targetIndex(centers, draggedIndex, draggedCenter) {
    return centers.filter((center, index) => index !== draggedIndex && center < draggedCenter).length;
}

function indicatorSlot(draggedIndex, target, count) {
    if (draggedIndex < 0 || target < 0 || target === draggedIndex || count < 2)
        return null;
    if (target >= count - 1)
        return {
            index: draggedIndex === count - 1 ? count - 2 : count - 1,
            below: true
        };
    return {
        index: target >= draggedIndex ? target + 1 : target,
        below: false
    };
}

function reordered(allIds, visibleIds, from, to) {
    return mergeOrder(allIds, moveItem(visibleIds, from, to));
}
