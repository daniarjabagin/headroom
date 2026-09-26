.pragma library

function halfDiagonal(width, height) {
    return Math.hypot(width / 2, height / 2);
}

function scale(innerDiameter, inset, width, height) {
    const room = innerDiameter / 2 - inset;
    const half = halfDiagonal(width, height);
    if (half <= 0)
        return 1;
    return Math.max(0, Math.min(1, room / half));
}
