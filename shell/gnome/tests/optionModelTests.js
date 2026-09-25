import { OptionModel } from '../src/prefs/optionModel.js';
import { check } from './check.js';

const MINUTE = { value: 60, label: 'Every minute' };
const FIVE = { value: 300, label: 'Every 5 minutes' };
const FIVE_RU = { value: 300, label: 'Каждые 5 минут' };

function modelWith(entries) {
    const model = new OptionModel();
    model.setOptions(entries);
    return model;
}

function testOptionChanges() {
    const model = new OptionModel();
    check('first options change', model.setOptions([MINUTE, FIVE]), true);
    check('same options unchanged', model.setOptions([{ ...MINUTE }, { ...FIVE }]), false);
    check('added option changes', model.setOptions([MINUTE, FIVE, { value: 45, label: 'Every 45 seconds' }]), true);
    check('removed option changes', model.setOptions([MINUTE, FIVE]), true);
    check('relabel changes', model.setOptions([MINUTE, FIVE_RU]), true);
    check('revalue changes', model.setOptions([MINUTE, { ...FIVE_RU, value: 600 }]), true);
    check('reorder changes', model.setOptions([{ ...FIVE_RU, value: 600 }, MINUTE]), true);
    check('labels kept', model.labels, ['Каждые 5 минут', 'Every minute']);
    check('empty after options changes', model.setOptions([]), true);
    check('empty again unchanged', model.setOptions([]), false);
}

function testOptionLookup() {
    const model = modelWith([MINUTE, FIVE]);
    check('index of value', model.indexOf(300), 1);
    check('unknown value selects first', model.indexOf(900), 0);
    check('value at index', model.valueAt(0), 60);
    check('invalid position', model.valueAt(4_294_967_295), null);
    check('negative position', model.valueAt(-1), null);
    check('empty model', new OptionModel().valueAt(0), null);
}

export function testOptionModel() {
    testOptionChanges();
    testOptionLookup();
}
