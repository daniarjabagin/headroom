import QtQuick
import org.kde.plasma.plasma5support as Plasma5Support

Plasma5Support.DataSource {
    id: runner

    signal exited(string command, int exitCode, string stdout)

    function run(command) {
        connectSource(command);
    }

    engine: "executable"
    connectedSources: []
    onNewData: (sourceName, data) => {
        disconnectSource(sourceName);
        runner.exited(sourceName, data["exit code"] ?? -1, data.stdout ?? "");
    }
}
