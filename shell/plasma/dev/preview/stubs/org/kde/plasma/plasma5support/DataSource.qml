import QtQuick

QtObject {
    id: source

    property string engine: ""
    property var connectedSources: []

    signal newData(string sourceName, var data)

    function connectSource(name) {
        connectedSources = connectedSources.concat([name]);
    }

    function disconnectSource(name) {
        connectedSources = connectedSources.filter(candidate => candidate !== name);
    }
}
