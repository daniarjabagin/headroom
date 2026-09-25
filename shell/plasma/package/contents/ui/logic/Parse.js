.pragma library

const HTTPS_URL = /^https:\/\/\S+$/;

function isObject(value) {
    return value !== null && typeof value === "object" && !Array.isArray(value);
}

function object(value) {
    return isObject(value) ? value : {};
}

function text(value) {
    return typeof value === "string" && value.length > 0 ? value : null;
}

function number(value) {
    return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function integer(value) {
    return Number.isInteger(value) ? value : null;
}

function count(value) {
    return number(value) ?? 0;
}

function list(value) {
    return Array.isArray(value) ? value.filter(isObject) : [];
}

function optionalList(value, parse) {
    return Array.isArray(value) ? list(value).map(parse) : null;
}

function timestamp(value) {
    const ms = typeof value === "string" ? Date.parse(value) : NaN;
    return Number.isNaN(ms) ? null : new Date(ms);
}

function oneOf(allowed, value, fallback) {
    return allowed.includes(value) ? value : fallback;
}

function httpsUrl(value) {
    const url = text(value);
    return url !== null && HTTPS_URL.test(url) ? url : null;
}
