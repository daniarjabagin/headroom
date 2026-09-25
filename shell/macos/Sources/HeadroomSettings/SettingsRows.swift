#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    extension SettingsContext {
        var release06: Bool { store.features.release06 }

        func binding<Value>(_ value: Value, _ change: @escaping (Value) -> SettingsChange) -> Binding<Value> {
            let store = store
            return Binding(get: { value }, set: { store.change(change($0)) })
        }
    }

    struct SettingToggle: View {
        let context: SettingsContext
        let title: String
        let detail: String
        let value: Bool
        let change: (Bool) -> SettingsChange

        init(
            _ context: SettingsContext, _ title: some LocalizedText, _ detail: some LocalizedText, _ value: Bool,
            _ change: @escaping (Bool) -> SettingsChange
        ) {
            self.context = context
            self.title = context.strings.text(title)
            self.detail = context.strings.text(detail)
            self.value = value
            self.change = change
        }

        var body: some View {
            Toggle(isOn: context.binding(value, change)) {
                TitledLabel(title: title, detail: detail)
            }
        }
    }

    struct SettingPicker<Value: Hashable, Options: RandomAccessCollection>: View where Options.Element == Value {
        let context: SettingsContext
        let title: String
        let detail: String?
        let value: Value
        let options: Options
        let label: (Value) -> String
        let change: (Value) -> SettingsChange
        var segmented = false

        var body: some View {
            let picker = Picker(selection: context.binding(value, change)) {
                ForEach(Array(options), id: \.self) { Text(label($0)).tag($0) }
            } label: {
                TitledLabel(title: title, detail: detail)
            }
            if segmented {
                picker.pickerStyle(.segmented)
            } else {
                picker
            }
        }
    }
#endif
