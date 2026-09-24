#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    enum AddAccountStep: Hashable {
        case methods(ProviderInfo)
        case flow(ProviderInfo, AddAccountMethod)
    }

    struct AddAccountSheet: View {
        static let size = CGSize(width: 480, height: 560)

        let context: SettingsContext
        let initialProvider: String?

        @Environment(\.dismiss) private var dismiss
        @State private var path: [AddAccountStep] = []
        @State private var routedInitialProvider = false

        var body: some View {
            let strings = context.strings
            NavigationStack(path: $path) {
                ProviderPickerView(context: context) { pick($0) }
                    .navigationTitle(strings.text(AddAccountText.addAccount))
                    .navigationDestination(for: AddAccountStep.self) { destination($0) }
            }
            .frame(width: Self.size.width, height: Self.size.height)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(strings.text(AddAccountText.close)) { dismiss() }
                }
            }
            .onChange(of: context.store.providers, initial: true) { routeInitialProvider() }
        }

        @ViewBuilder
        private func destination(_ step: AddAccountStep) -> some View {
            switch step {
            case .methods(let provider):
                MethodPickerView(context: context, provider: provider) { path.append(.flow(provider, $0)) }
            case .flow(let provider, let method):
                FlowView(context: context, provider: provider, method: method) { dismiss() }
            }
        }

        private func pick(_ provider: ProviderInfo) {
            let methods = provider.supportedMethods
            if methods.count == 1, let method = methods.first {
                path.append(.flow(provider, method))
            } else {
                path.append(.methods(provider))
            }
        }

        private func routeInitialProvider() {
            guard !routedInitialProvider, let initialProvider, case .loaded(let providers) = context.store.providers
            else { return }
            routedInitialProvider = true
            guard let provider = providers.first(where: { $0.id == initialProvider }) else { return }
            pick(provider)
        }
    }

    struct ProviderPickerView: View {
        let context: SettingsContext
        let onPick: (ProviderInfo) -> Void

        var body: some View {
            let strings = context.strings
            switch context.store.providers {
            case .loading:
                ProgressView(strings.text(AddAccountText.loadingProviders))
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            case .failed(let reason):
                unavailable(reason)
            case .loaded(let providers) where providers.isEmpty:
                unavailable(strings.text(AddAccountText.noProvidersDetail))
            case .loaded(let providers):
                ChoiceList(heading: strings.text(AddAccountText.chooseService)) {
                    ForEach(providers) { provider in
                        ChoiceRow(
                            title: provider.displayName,
                            subtitle: provider.supportedMethods.map { $0.summary(strings) }.joined(separator: " · ")
                        ) {
                            ProviderMark(
                                context: context, providerID: provider.id, name: provider.displayName, size: 24)
                        } action: {
                            onPick(provider)
                        }
                    }
                }
            }
        }

        private func unavailable(_ reason: String) -> some View {
            ContentUnavailableView(
                context.strings.text(AddAccountText.noProviders), systemImage: "exclamationmark.triangle",
                description: Text(reason))
        }
    }

    struct MethodPickerView: View {
        let context: SettingsContext
        let provider: ProviderInfo
        let onPick: (AddAccountMethod) -> Void

        var body: some View {
            let strings = context.strings
            ChoiceList(heading: strings.fill(AddAccountText.methodQuestion, ["provider": provider.displayName])) {
                ForEach(provider.supportedMethods, id: \.self) { method in
                    ChoiceRow(title: method.summary(strings), subtitle: nil) {
                        EmptyView()
                    } action: {
                        onPick(method)
                    }
                }
            }
            .navigationTitle(provider.displayName)
        }
    }

    struct ChoiceList<Rows: View>: View {
        let heading: String
        @ViewBuilder let rows: Rows

        var body: some View {
            Form {
                Section {
                    rows
                } header: {
                    Text(heading)
                }
            }
            .formStyle(.grouped)
        }
    }

    struct ChoiceRow<Icon: View>: View {
        let title: String
        let subtitle: String?
        @ViewBuilder let icon: Icon
        let action: () -> Void

        var body: some View {
            Button(action: action) {
                HStack(spacing: 10) {
                    icon
                    VStack(alignment: .leading, spacing: 2) {
                        Text(title)
                        if let subtitle { Text(subtitle).font(.caption).foregroundStyle(.secondary) }
                    }
                    Spacer()
                    Image(systemName: "chevron.right").foregroundStyle(.tertiary)
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
        }
    }
#endif
