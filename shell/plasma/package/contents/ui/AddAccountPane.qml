pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import "logic/I18n.js" as I18n
import "logic/Registry.js" as Registry

SettingsGroup {
    id: pane

    required property var providers
    required property string lang
    required property bool providersRequested
    required property string launchedProvider
    property bool animated: true
    property string selectedProvider: ""
    readonly property var chosenProvider: Registry.findProvider(providers, selectedProvider) ?? providers[0] ?? null

    signal addRequested(var provider, string label)
    signal providerPicked

    objectName: "addAccountPane"
    title: I18n.tr(lang, "Add Account")
    description: I18n.tr(lang, "Pick a service. Accounts added here never touch the one your CLI uses.")

    SettingsRow {
        visible: pane.chosenProvider === null
        separated: false
        title: pane.providersRequested ? I18n.tr(pane.lang, "No providers available") : I18n.tr(pane.lang, "Loading…")
    }

    ProviderPicker {
        visible: pane.chosenProvider !== null
        providers: pane.providers
        selected: pane.chosenProvider?.id ?? ""
        animated: pane.animated
        onPicked: providerId => {
            pane.selectedProvider = providerId;
            pane.providerPicked();
        }
    }

    Loader {
        Layout.fillWidth: true
        active: pane.chosenProvider !== null

        sourceComponent: AddAccountRow {
            provider: pane.chosenProvider
            lang: pane.lang
            launched: pane.launchedProvider === pane.chosenProvider.id
            onAddRequested: label => pane.addRequested(pane.chosenProvider, label)
        }
    }
}
