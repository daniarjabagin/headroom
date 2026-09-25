#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import SwiftUI

    struct WelcomeFoundView: View {
        static let listHeight: CGFloat = 300

        let flow: WelcomeFlow
        let model: AppModel
        let store: SettingsStore
        @State private var icons = ProviderIconStore()

        var body: some View {
            VStack(spacing: 0) {
                Image(nsImage: NSApp.applicationIconImage).resizable().frame(width: 64, height: 64).padding(.bottom, 14)
                heading.padding(.bottom, 20)
                found.padding(.bottom, 24)
                buttons
            }
            .environment(icons)
        }

        private var strings: UIStrings { model.formatter.strings }

        private var rows: [OnboardingRow] {
            let providers: [ProviderInfo]
            if case .loaded(let loaded) = store.providers { providers = loaded } else { providers = [] }
            return OnboardingRows.rows(accounts: model.orderedAccounts, providers: providers, strings: strings)
        }

        private var heading: some View {
            VStack(spacing: 6) {
                Text(strings.text(OnboardingText.welcome)).font(.title2.weight(.semibold))
                Text(strings.text(OnboardingText.tagline)).foregroundStyle(.secondary)
            }
            .multilineTextAlignment(.center)
            .fixedSize(horizontal: false, vertical: true)
        }

        private var found: some View {
            VStack(alignment: .leading, spacing: 4) {
                Text(strings.text(OnboardingText.whatWeFound)).font(.headline)
                Text(strings.text(OnboardingText.whatWeFoundDetail)).font(.callout).foregroundStyle(.secondary)
                    .padding(.bottom, 6)
                ScrollView {
                    VStack(spacing: 0) {
                        if rows.isEmpty {
                            Text(strings.text(OnboardingText.nothingFound))
                                .font(.callout)
                                .foregroundStyle(.secondary)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(12)
                            Divider()
                        }
                        ForEach(rows) { row in
                            WelcomeFoundRow(row: row, model: model)
                            Divider().padding(.leading, 44)
                        }
                        addAnother
                    }
                }
                .frame(maxHeight: Self.listHeight)
                .fixedSize(horizontal: false, vertical: true)
                .background(Palette.card, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
            }
        }

        private var addAnother: some View {
            Button {
                model.settingsPresenter?(.addAccount(provider: nil))
            } label: {
                HStack(spacing: 12) {
                    Image(systemName: "plus").frame(width: 20)
                    Text(strings.text(OnboardingText.addAnotherAccount))
                    Spacer()
                    Image(systemName: "chevron.right").foregroundStyle(.tertiary)
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 10)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
        }

        private var buttons: some View {
            VStack(spacing: 8) {
                Button(strings.text(OnboardingText.start)) { flow.start() }
                    .controlSize(.large)
                    .buttonStyle(.borderedProminent)
                    .keyboardShortcut(.defaultAction)
                Button(strings.text(OnboardingText.chooseLater)) { flow.chooseLater() }
                    .buttonStyle(.link)
            }
        }
    }

    struct WelcomeFoundRow: View {
        let row: OnboardingRow
        let model: AppModel

        var body: some View {
            HStack(spacing: 12) {
                ProviderGlyph(provider: row.providerID, size: 20)
                VStack(alignment: .leading, spacing: 2) {
                    Text(row.title).lineLimit(1)
                    Text(row.subtitle).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                }
                Spacer(minLength: 8)
                trailing
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .opacity(row.kind == .notInstalled ? 0.45 : 1)
        }

        @ViewBuilder
        private var trailing: some View {
            switch row.kind {
            case .tracked(let accountID, let shown):
                Toggle(
                    row.title,
                    isOn: Binding(
                        get: { shown },
                        set: { model.send(.setAccountHidden(accountID: accountID, hidden: !$0)) })
                )
                .labelsHidden()
                .toggleStyle(.switch)
            case .signIn(let provider):
                Button(model.formatter.strings.text(OnboardingText.signIn)) { model.signIn(provider: provider) }
            case .notInstalled:
                Toggle(row.title, isOn: .constant(false)).labelsHidden().toggleStyle(.switch).disabled(true)
            }
        }
    }
#endif
