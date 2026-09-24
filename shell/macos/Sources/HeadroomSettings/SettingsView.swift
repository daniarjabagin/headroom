#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct SettingsPlaceholder: View {
        let context: SettingsContext

        var body: some View {
            let strings = context.strings
            switch context.model.phase {
            case .starting, .connecting, .connected:
                ProgressView(strings.text(ServiceText.connectingToHeadroom))
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            case .disconnected:
                ContentUnavailableView(
                    strings.text(SettingsText.settingsUnavailable), systemImage: "bolt.horizontal.circle",
                    description: Text(strings.text(SettingsText.settingsUnavailableDetail)))
            case .incompatible(let text):
                ContentUnavailableView(
                    strings.text(SettingsText.settingsUnavailable), systemImage: "exclamationmark.triangle",
                    description: Text(strings.text(text)))
            }
        }
    }

    struct TitledLabel: View {
        let title: String
        let detail: String?

        var body: some View {
            Text(title)
            if let detail { Text(detail) }
        }
    }

    struct ProviderMark: View {
        let context: SettingsContext
        let providerID: String
        let name: String
        let size: CGFloat

        var body: some View {
            if let image = context.image(forProvider: providerID) {
                image.resizable().scaledToFit().frame(width: size, height: size)
            } else {
                Text(String(name.prefix(1)).uppercased())
                    .font(.system(size: size * 0.5, weight: .semibold))
                    .foregroundStyle(.secondary)
                    .frame(width: size, height: size)
                    .background(.quaternary, in: RoundedRectangle(cornerRadius: size * 0.25, style: .continuous))
            }
        }
    }
#endif
