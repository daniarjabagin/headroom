.pragma library

function idle() {
    return {
        pending: [],
        busy: false
    };
}

function enqueue(queue, patch) {
    if (queue.busy)
        return {
            queue: {
                pending: queue.pending.concat([patch]),
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
