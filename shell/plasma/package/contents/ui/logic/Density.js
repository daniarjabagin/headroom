.pragma library

.import "Metrics.js" as Metrics

const COMPACT_FONT_STEP = 1;

function isCompact(display) {
    return display?.density === "compact";
}

function sectionGap(units, compact) {
    return compact ? units.largeSpacing : Metrics.sectionGap(units);
}

function headerGap(units, compact) {
    return compact ? units.smallSpacing / 2 : units.smallSpacing;
}

function cardGutter(units, compact) {
    return compact ? Math.round(units.smallSpacing * 0.75) : Metrics.gutter(units);
}

function barRowTop(units, compact) {
    return compact ? Math.round(units.smallSpacing * 1.25) : Metrics.barRowPadding(units);
}

function barRowBottom(units, compact) {
    return compact ? units.mediumSpacing : Metrics.barRowPadding(units);
}

function meterHeight(units, compact) {
    return compact ? units.smallSpacing : Metrics.meterHeight(units);
}

function donutSize(units, compact) {
    return compact ? Math.round(units.gridUnit * 4.67) : Metrics.donutSize(units);
}

function providerIcon(units, compact) {
    return compact ? units.iconSizes.small - 2 : units.iconSizes.small;
}

function trendHeight(units, compact) {
    return compact ? Math.round(units.gridUnit * 0.75) : Metrics.trendHeight(units);
}

function spendPadding(units, compact) {
    return compact ? units.largeSpacing : Metrics.cardPadding(units);
}

function fontStep(compact) {
    return compact ? COMPACT_FONT_STEP : 0;
}
