import QtQuick
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import "logic/Tokens.js" as Tokens

PlasmaComponents3.Label {
    property string role: "body"
    property string emphasis: "primary"
    property int weight: role === "title" || role === "label" ? Font.DemiBold : Font.Normal
    property real step: 0

    function pointSizeFor(role) {
        const base = Kirigami.Theme.defaultFont.pointSize;
        const small = Kirigami.Theme.smallFont.pointSize;
        switch (role) {
        case "title":
            return base + 0.5;
        case "label":
            return base;
        case "caption":
            return small;
        case "micro":
            return small - 1;
        default:
            return base - 1;
        }
    }

    function colorFor(emphasis) {
        if (emphasis === "secondary")
            return Tokens.secondaryText(Kirigami.Theme);
        if (emphasis === "tertiary")
            return Tokens.tertiaryText(Kirigami.Theme);
        return Kirigami.Theme.textColor;
    }

    color: colorFor(emphasis)
    textFormat: Text.PlainText
    font.family: Kirigami.Theme.defaultFont.family
    font.pointSize: pointSizeFor(role) - step
    font.weight: weight
    font.features: {
        "tnum": 1
    }
}
