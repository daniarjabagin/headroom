import QtQuick
import org.kde.kquickcontrols as KQuickControls
import "logic/Accelerator.js" as Accelerator
import "logic/AcceleratorEncode.js" as AcceleratorEncode
import "logic/Settings.js" as Settings

SettingsGroup {
    id: group

    required property ConfigScaffold page
    readonly property string accelerator: page.current.shortcuts.open
    property bool rejected: false

    function save(sequence) {
        const encoded = AcceleratorEncode.encode(sequence);
        rejected = encoded === null;
        if (encoded !== null && encoded !== accelerator)
            page.updateSettings(Settings.shortcutPatch(encoded));
        recorder.keySequence = Qt.binding(() => Accelerator.keySequence(group.accelerator));
    }

    title: page.tr("Keyboard")
    description: rejected ? page.tr("Headroom can't store this shortcut. Try a letter, digit or function key with modifiers.") : page.tr("This shortcut replaces the one on the widget's Keyboard Shortcuts page.")

    SettingsRow {
        separated: false
        title: group.page.tr("Open Headroom")
        subtitle: group.page.tr("Opens the popup from any app")

        KQuickControls.KeySequenceItem {
            id: recorder

            objectName: "shortcutRecorder"
            keySequence: Accelerator.keySequence(group.accelerator)
            modifierlessAllowed: false
            multiKeyShortcutsAllowed: false
            showClearButton: true
            onCaptureFinished: group.save(String(recorder.keySequence))
        }
    }
}
