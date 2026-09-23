.pragma library

function popupWidth(units) {
    return units.gridUnit * 18;
}

function gutter(units) {
    return Math.round(units.smallSpacing * 1.25);
}

function rowInset(units) {
    return Math.round(units.gridUnit * 0.75);
}

function sectionGap(units) {
    return rowInset(units);
}

function barRowPadding(units) {
    return units.largeSpacing + units.smallSpacing / 2;
}

function textRowPadding(units) {
    return units.mediumSpacing;
}

function headerInset(units) {
    return units.largeSpacing + units.smallSpacing / 2;
}

function cardPadding(units) {
    return units.largeSpacing + units.smallSpacing;
}

function cardRadius(units) {
    return units.largeSpacing + units.smallSpacing;
}

function noticeRadius(units) {
    return Math.round(units.smallSpacing * 2.25);
}

function chipRadius(units) {
    return Math.round(units.smallSpacing * 1.5);
}

function buttonRadius(units) {
    return Math.round(units.smallSpacing * 1.25);
}

function controlRadius(units) {
    return units.smallSpacing * 2;
}

function donutGap(units) {
    return units.smallSpacing * 0.75;
}

function meterHeight(units) {
    return Math.round(units.smallSpacing * 1.25);
}

function tickOverhang(units) {
    return units.smallSpacing / 2;
}

function hairline(units) {
    return Math.max(1, Math.round(units.smallSpacing / 4));
}

function tinyIcon(units) {
    return Math.round(units.iconSizes.small * 0.6875);
}

function caretIcon(units) {
    return Math.round(units.smallSpacing * 2.5);
}

function tileSize(units) {
    return units.smallSpacing * 6;
}

function donutSize(units) {
    return Math.round(units.gridUnit * 5.75);
}

function controlHeight(units) {
    return units.smallSpacing * 7;
}

function trendHeight(units) {
    return units.gridUnit;
}
