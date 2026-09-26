import QtCore
import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/AdvancedPrefs.js" as AdvancedPrefs
import "logic/Options.js" as Options
import "logic/Settings.js" as Settings

ConfigScaffold {
    id: page

    readonly property bool capable: daemon.supports06
    readonly property real controlWidth: Kirigami.Units.gridUnit * 13
    readonly property int copiedMs: 2000
    property var diagnostics: null
    property var failure: null
    property string copied: ""

    function loadDiagnostics(onLoaded) {
        daemon.getDiagnostics((failure, value) => {
            page.failure = failure;
            if (value !== null)
                page.diagnostics = value;
            if (value !== null && onLoaded)
                onLoaded(value);
        });
    }

    function copy(kind, text) {
        clipboard.text = text;
        clipboard.selectAll();
        clipboard.copy();
        copied = kind;
        copiedReset.restart();
    }

    function copyDiagnostics() {
        loadDiagnostics(value => page.copy("diagnostics", value.text));
    }

    onCapableChanged: {
        if (capable && diagnostics === null)
            loadDiagnostics(null);
    }

    TextLabel {
        visible: !page.capable
        Layout.fillWidth: true
        emphasis: "secondary"
        wrapMode: Text.Wrap
        text: page.tr("Needs a newer Headroom service")
    }

    SettingsGroup {
        visible: page.capable
        title: page.tr("Logging")

        SettingsRow {
            separated: false
            title: page.tr("Log level")
            subtitle: AdvancedPrefs.logLevelNote(page.lang, page.diagnostics)

            OptionCombo {
                objectName: "logLevel"
                Layout.preferredWidth: page.controlWidth
                options: Options.logLevelOptions(page.lang)
                value: page.current.logging.level
                onPicked: value => page.updateSettings(Settings.loggingPatch(value))
            }
        }

        SettingsRow {
            objectName: "logFileRow"
            title: page.tr("Log file")
            subtitle: AdvancedPrefs.logFileLine(page.lang, page.diagnostics, page.failure)

            IconButton {
                objectName: "copyLogPath"
                enabled: (page.diagnostics?.logFile ?? null) !== null
                iconName: page.copied === "path" ? "checkmark" : "edit-copy"
                text: page.tr("Copy path")
                onClicked: page.copy("path", page.diagnostics.logFile)
            }

            IconButton {
                objectName: "openLogFolder"
                enabled: AdvancedPrefs.folderUrl(page.diagnostics?.logFile, home.location) !== ""
                iconName: "folder-open"
                text: page.tr("Open folder")
                onClicked: Qt.openUrlExternally(AdvancedPrefs.folderUrl(page.diagnostics.logFile, home.location))
            }
        }
    }

    SettingsGroup {
        visible: page.capable
        title: page.tr("Troubleshooting")
        description: page.tr("Paste the diagnostics into a bug report.")

        SettingsRow {
            separated: false
            title: page.tr("Copy diagnostics")
            subtitle: page.tr("Versions, desktop and account states. No tokens or emails.")

            QQC2.Button {
                objectName: "copyDiagnostics"
                text: page.copied === "diagnostics" ? page.tr("Copied") : page.tr("Copy")
                onClicked: page.copyDiagnostics()
            }
        }
    }

    SettingsGroup {
        visible: page.capable

        SettingsRow {
            separated: false
            title: page.tr("Reset all settings")
            subtitle: page.tr("Accounts stay signed in")

            QQC2.Button {
                objectName: "resetSettings"
                text: page.tr("Reset…")
                icon.name: "edit-reset"
                onClicked: resetDialog.open()
            }
        }
    }

    SettingsGroup {
        title: page.tr("Support Headroom")

        SettingsRow {
            separated: false
            title: page.tr("Star Headroom on GitHub")
            subtitle: page.tr("Stars help other people find it. It's free and takes a second.")

            QQC2.Button {
                objectName: "openGitHub"
                text: page.tr("Open GitHub")
                icon.name: "link"
                onClicked: Qt.openUrlExternally(AdvancedPrefs.REPOSITORY_URL)
            }
        }
    }

    ResetDialog {
        id: resetDialog

        lang: page.lang
        onResetConfirmed: page.daemon.resetSettings()
    }

    TextEdit {
        id: clipboard

        visible: false
    }

    Timer {
        id: copiedReset

        interval: page.copiedMs
        onTriggered: page.copied = ""
    }

    QtObject {
        id: home

        readonly property url location: StandardPaths.writableLocation(StandardPaths.HomeLocation)
    }
}
