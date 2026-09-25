import { modelBreakdown } from '../src/breakdown.js';
import { mergeOrder, moveItem } from '../src/order.js';
import { parseProgressLine } from '../src/progress.js';
import { check } from './check.js';

function model(name, totalTokens, costMicros, partial = false) {
    return { model: name, totalTokens, costMicros, partial };
}

function brief(entry) {
    return [entry.name, entry.detail, entry.tokens, entry.exactTokens, entry.cost, entry.partial, entry.sharePermille];
}

export function testModelBreakdown() {
    check('no models', modelBreakdown([], null), null);
    const few = modelBreakdown([model('gpt-5.5', 3_000_000, 11_210_000), model('next', 412_000, 0, true)], null);
    check('few rows', few.rows.map(brief), [
        ['gpt-5.5', null, '3M', '3,000,000', '$11.21', false, 0],
        ['next', null, '412K', '412,000', 'unpriced', true, 0],
    ]);
    check('few partial', few.partial, true);
    check('tokens text', few.rows[0].tokensText, '3M tokens');
    const six = Array.from({ length: 6 }, (_, index) => model(`m${index}`, 1000, 1_000_000));
    check('all given shown', modelBreakdown(six, null).rows.length, 6);
    const five = Array.from({ length: 5 }, (_, index) => model(`m${index}`, 1000 * (index + 1), 10_000 * (index + 1)));
    const other = { count: 3, totalTokens: 21_000, costMicros: 210_000, partial: false };
    const shaped = modelBreakdown(five, other);
    check(
        'top five plus other',
        shaped.rows.map(entry => [entry.name, entry.detail]),
        [
            ['m0', null],
            ['m1', null],
            ['m2', null],
            ['m3', null],
            ['m4', null],
            ['Other', '· 3 models'],
        ]
    );
    check('other from daemon', [shaped.rows[5].exactTokens, shaped.rows[5].cost], ['21,000', '$0.21']);
    check('other partial', shaped.partial, false);
    const unpricedOther = modelBreakdown(five, { ...other, costMicros: 0, partial: true });
    check('other unpriced', [unpricedOther.rows[5].cost, unpricedOther.partial], ['unpriced', true]);
    check('partial with cost', modelBreakdown([model('mixed', 10, 1_500_000, true)], null).rows[0].cost, '$1.50');
}

export function testModelShares() {
    const five = Array.from({ length: 5 }, (_, index) => model(`m${index}`, 1000 * (index + 1), 10_000 * (index + 1)));
    const other = { count: 3, totalTokens: 21_000, costMicros: 210_000, partial: false };
    const totals = { costMicros: 360_000, totalTokens: 36_000 };
    const shares = modelBreakdown(five, other, totals).rows.map(entry => entry.sharePermille);
    check('shares by cost, rounded down', shares, [27, 55, 83, 111, 138, 583]);
    const free = modelBreakdown([model('a', 1, 0), model('b', 3, 0)], null, { costMicros: 0, totalTokens: 4 });
    check(
        'shares by tokens without cost',
        free.rows.map(entry => entry.sharePermille),
        [250, 750]
    );
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
        version: null,
        relogin: false,
    });
    check('progress update done', parseProgressLine('{"event":"done","version":"0.5.0","relogin":true}'), {
        event: 'done',
        accountId: null,
        version: '0.5.0',
        relogin: true,
    });
    check('progress step', parseProgressLine('{"event":"step","text":"Downloading"}'), {
        event: 'step',
        text: 'Downloading',
    });
    check('progress empty step', parseProgressLine('{"event":"step","text":""}'), {
        event: 'output',
        line: '{"event":"step","text":""}',
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
