export const failures = [];

export function check(name, actual, expected) {
    if (JSON.stringify(actual) !== JSON.stringify(expected))
        failures.push(`${name}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
}

export function throws(name, run, type) {
    try {
        run();
        failures.push(`${name}: expected ${type.name}`);
    } catch (error) {
        if (!(error instanceof type)) failures.push(`${name}: threw ${error}`);
    }
}
