pragma Singleton

import QtQuick
import headroom.preview

QtObject {
    id: bus

    property var messages: []
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
        if (message.member === "UpdateSettings")
            PreviewConfig.applySettingsPatch(message.arguments[0]);
        return {
            value: null
        };
    }

    function asyncCall(message) {
        messages = messages.concat([message]);
        const reply = replyComponent.createObject(bus, answer(message)) as PendingReply;
        if (!(message.member === "GetState" && PreviewConfig.scenario === "loading"))
            reply.start();
        return reply;
    }
}
