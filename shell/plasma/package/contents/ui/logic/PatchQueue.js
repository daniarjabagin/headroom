.pragma library

function idle() {
    return {
        pending: [],
        busy: false
    };
}

function isObject(value) {
    return value !== null && typeof value === "object" && !Array.isArray(value);
}

function composable(first, second) {
    if (!isObject(first) || !isObject(second))
        return false;
    return Object.entries(second).every(([key, value]) => !isObject(value) || first[key] === undefined || composable(first[key], value));
}

function compose(first, second) {
    const result = Object.assign({}, first);
    for (const [key, value] of Object.entries(second))
        result[key] = isObject(value) && isObject(first[key]) ? compose(first[key], value) : value;
    return result;
}

function appended(pending, patch) {
    const last = pending[pending.length - 1];
    if (pending.length > 0 && composable(last, patch))
        return pending.slice(0, -1).concat([compose(last, patch)]);
    return pending.concat([patch]);
}

function enqueue(queue, patch) {
    if (queue.busy)
        return {
            queue: {
                pending: appended(queue.pending, patch),
                busy: true
            },
            send: null,
            drained: false
        };
    return {
        queue: {
            pending: [],
            busy: true
        },
        send: patch,
        drained: false
    };
}

function settle(queue) {
    if (queue.pending.length === 0)
        return {
            queue: idle(),
            send: null,
            drained: true
        };
    return {
        queue: {
            pending: queue.pending.slice(1),
            busy: true
        },
        send: queue.pending[0],
        drained: false
    };
}
