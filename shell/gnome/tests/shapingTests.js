import { modelBreakdown } from '../src/breakdown.js';
import { mergeOrder, moveItem } from '../src/order.js';
import { parseProgressLine } from '../src/prefs/progress.js';
import { check } from './check.js';

function model(name, totalTokens, costMicros, partial = false) {
    return { model: name, totalTokens, costMicros, partial };
}

export function testModelBreakdown() {
    check('no models', modelBreakdown([]), null);
    const few = modelBreakdown([model('gpt-5.5', 3_000_000, 11_210_000), model('next', 412_000, 0, true)]);
    check('few rows', few.rows, [
        { name: 'gpt-5.5', tokens: '3M', exactTokens: '3,000,000', cost: '$11.21', partial: false },
        { name: 'next', tokens: '412K', exactTokens: '412,000', cost: 'unpriced', partial: true },
    ]);
    check('few partial', few.partial, true);
    const six = Array.from({ length: 6 }, (_, index) => model(`m${index}`, 1000, 1_000_000));
    check('six shown', modelBreakdown(six).rows.length, 6);
    const eight = Array.from({ length: 8 }, (_, index) => model(`m${index}`, 1000 * (index + 1), 10_000 * (index + 1)));
    const shaped = modelBreakdown(eight);
    check(
        'top five plus other',
        shaped.rows.map(entry => entry.name),
        ['m0', 'm1', 'm2', 'm3', 'm4', 'Other (3)']
    );
    check('other sums', [shaped.rows[5].exactTokens, shaped.rows[5].cost], ['21,000', '$0.21']);
    check('other partial', shaped.rows[5].partial, false);
    check('partial with cost', modelBreakdown([model('mixed', 10, 1_500_000, true)]).rows[0].cost, '$1.50');
}

export function testOrder() {
    check('move down', moveItem(['a', 'b', 'c'], 0, 2), ['b', 'c', 'a']);
    check('move up', moveItem(['a', 'b', 'c'], 2, 0), ['c', 'a', 'b']);
    check('move same', moveItem(['a', 'b'], 1, 1), ['a', 'b']);
    check('move out of range', moveItem(['a', 'b'], 5, 0), ['a', 'b']);
    check('merge keeps hidden slots', mergeOrder(['a', 'h', 'b', 'c'], ['c', 'a', 'b']), ['c', 'h', 'a', 'b']);
}

export function testProgress() {
    check('progress url', parseProgressLine('{"event":"url","url":"https://x"}'), { event: 'url', url: 'https://x' });
    check('progress done', parseProgressLine('{"event":"done","account_id":"codex:1"}'), {
        event: 'done',
        accountId: 'codex:1',
    });
    check('progress error', parseProgressLine('{"event":"error","message":"no"}'), { event: 'error', message: 'no' });
    check('progress output', parseProgressLine('{"event":"output","line":"hi"}'), { event: 'output', line: 'hi' });
    check('progress started', parseProgressLine('{"event":"started","provider":"codex"}'), { event: 'started' });
    check('progress plain', parseProgressLine('warning: plain text'), { event: 'output', line: 'warning: plain text' });
    check('progress unknown', parseProgressLine('{"event":"mystery"}'), {
        event: 'output',
        line: '{"event":"mystery"}',
    });
    check('progress blank', parseProgressLine('   '), null);
    check('progress url missing', parseProgressLine('{"event":"url"}'), { event: 'output', line: '{"event":"url"}' });
}
