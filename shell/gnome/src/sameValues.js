export function sameValues(a, b) {
    return a.length === b.length && a.every((value, index) => value === b[index]);
}
