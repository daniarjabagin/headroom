function sameList(left, right) {
    return left.length === right.length && left.every((item, index) => item === right[index]);
}

export class OptionModel {
    constructor() {
        this.values = [];
        this.labels = [];
    }

    setOptions(entries) {
        const values = entries.map(entry => entry.value);
        const labels = entries.map(entry => entry.label);
        if (sameList(values, this.values) && sameList(labels, this.labels)) return false;
        this.values = values;
        this.labels = labels;
        return true;
    }

    indexOf(value) {
        return Math.max(0, this.values.indexOf(value));
    }

    valueAt(index) {
        return index >= 0 && index < this.values.length ? this.values[index] : null;
    }
}
