pragma Singleton

import QtQuick
import headroom.preview

QtObject {
    id: bus

    property var calls: []
    readonly property Component replyComponent: Component {
        PendingReply {}
    }

    function answer(message) {
        if (message.member === "GetState")
            return {
                value: PreviewConfig.shiftedState()
            };
        if (message.member === "GetSettings")
            return {
                value: PreviewConfig.settingsJson()
            };
        return {
            value: null
        };
    }

    function asyncCall(message) {
        calls = calls.concat([message.member]);
        const reply = replyComponent.createObject(bus, answer(message)) as PendingReply;
        if (!(message.member === "GetState" && PreviewConfig.scenario === "loading"))
            reply.start();
        return reply;
    }
}
