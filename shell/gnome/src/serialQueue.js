export class SerialQueue {
    constructor() {
        this._tail = Promise.resolve();
    }

    push(task) {
        const run = this._tail.then(task);
        this._tail = run.catch(() => null);
        return run;
    }
}
