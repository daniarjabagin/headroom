import QtQuick

Timer {
    id: reply

    property bool isFinished: false
    property bool isError: false
    property var value: null
    property var error: ({
            message: ""
        })

    signal finished

    interval: 1
    onTriggered: {
        isFinished = true;
        reply.finished();
    }
}
