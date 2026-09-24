.pragma library

const MICROS_PER_CENT = 10000;
const CURRENCY_SYMBOLS = {
    USD: "$",
    CNY: "¥",
    EUR: "€"
};

function grouped(value) {
    return String(value).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}

function centsOf(micros) {
    const cents = Math.round(Math.abs(micros) / MICROS_PER_CENT);
    return micros < 0 ? -cents : cents;
}

function money(currency, micros) {
    const cents = centsOf(micros);
    const whole = Math.abs(cents);
    const digits = `${grouped(Math.trunc(whole / 100))}.${String(whole % 100).padStart(2, "0")}`;
    const sign = cents < 0 ? "-" : "";
    const symbol = CURRENCY_SYMBOLS[currency];
    return symbol ? `${sign}${symbol}${digits}` : `${sign}${digits} ${currency}`;
}
