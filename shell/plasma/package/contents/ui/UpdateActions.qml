import QtQuick
import "logic/Update.js" as Update

Item {
    id: actions

    property var update: null
    property var run: Update.IDLE
    property string copiedCommand: ""
    readonly property string kind: update !== null ? Update.action(update) : ""
    readonly property bool copied: update !== null && update.command !== null && update.command === copiedCommand

    function install() {
        if (run.phase === "running")
            return;
        run = Update.started();
        runner.run(Update.COMMAND);
    }

    function copyCommand() {
        if (update?.command === null || update?.command === undefined)
            return;
        clipboard.text = update.command;
        clipboard.selectAll();
        clipboard.copy();
        copiedCommand = update.command;
    }

    function openRelease() {
        if (update?.url)
            Qt.openUrlExternally(update.url);
    }

    function trigger() {
        if (kind === "install")
            install();
        else if (kind === "command")
            copyCommand();
        else if (kind === "notes")
            openRelease();
    }

    function finish(command, exitCode, stdout) {
        if (command === Update.COMMAND)
            run = Update.outcome(stdout, exitCode);
    }

    visible: false

    CommandRunner {
        id: runner

        objectName: "updateRunner"
        onExited: (command, exitCode, stdout) => actions.finish(command, exitCode, stdout)
    }

    TextEdit {
        id: clipboard

        visible: false
    }
}
