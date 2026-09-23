import { SerialQueue } from '../src/serialQueue.js';
import { check } from './check.js';

async function ticks(count) {
    for (let index = 0; index < count; index += 1) await Promise.resolve();
}

export async function testSerialQueue() {
    const queue = new SerialQueue();
    const order = [];
    const slow = queue.push(async () => {
        await ticks(20);
        order.push('slow');
    });
    const failing = queue.push(async () => {
        order.push('failing');
        throw new Error('rejected');
    });
    const fast = queue.push(() => order.push('fast'));
    await slow;
    const failure = await failing.catch(error => error.message);
    await fast;
    check('queue order', order, ['slow', 'failing', 'fast']);
    check('queue failure surfaces', failure, 'rejected');
}
