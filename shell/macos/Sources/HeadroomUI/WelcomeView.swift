#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    public struct WelcomeView: View {
        public static let width: CGFloat = 420

        @Bindable var flow: WelcomeFlow
        let model: AppModel
        let store: SettingsStore
        let reducedMotion: Bool
        @Environment(\.accessibilityReduceMotion) private var systemReducedMotion

        public init(flow: WelcomeFlow, model: AppModel, store: SettingsStore, reducedMotion: Bool) {
            self.flow = flow
            self.model = model
            self.store = store
            self.reducedMotion = reducedMotion
        }

        public var body: some View {
            Group {
                switch flow.step {
                case .found:
                    WelcomeFoundView(flow: flow, model: model, store: store).padding(.top, 16)
                case .menuBar:
                    menuBarStep.transition(.opacity)
                }
            }
            .padding(.horizontal, 28)
            .padding(.top, 44)
            .padding(.bottom, 20)
            .frame(width: Self.width)
            .fixedSize(horizontal: false, vertical: true)
            .animation(Motion.animation(Motion.fade, reduced: motionReduced), value: flow.step)
            .environment(\.headroomReducedMotion, motionReduced)
        }

        private var menuBarStep: some View {
            VStack(spacing: 0) {
                WelcomeIllustration(reducedMotion: motionReduced)
                    .padding(.bottom, 4)
                heading
                    .padding(.bottom, 20)
                loginCard
                    .padding(.bottom, 12)
                accountsNote
                    .padding(.bottom, 24)
                buttons
            }
        }

        private var strings: UIStrings { model.formatter.strings }

        private var motionReduced: Bool { reducedMotion || systemReducedMotion }

        private var heading: some View {
            VStack(spacing: 8) {
                Text(strings.text(WelcomeText.title))
                    .font(.title2.weight(.semibold))
                Text(strings.text(WelcomeText.menuBarHint))
                    .font(.body)
                    .foregroundStyle(.secondary)
            }
            .multilineTextAlignment(.center)
            .fixedSize(horizontal: false, vertical: true)
        }

        private var loginCard: some View {
            VStack(alignment: .leading, spacing: 8) {
                Toggle(isOn: $flow.openAtLogin) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(strings.text(WelcomeText.openAtLogin)).font(.body.weight(.medium))
                        Text(strings.text(WelcomeText.openAtLoginDetail))
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                .toggleStyle(.switch)
                if let failure = flow.failure {
                    Label(failure, systemImage: "exclamationmark.triangle.fill")
                        .font(.caption)
                        .foregroundStyle(Palette.errorForeground)
                        .transition(.opacity)
                }
            }
            .fixedSize(horizontal: false, vertical: true)
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Palette.card, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
            .animation(Motion.animation(Motion.fade, reduced: motionReduced), value: flow.failure)
        }

        private var accountsNote: some View {
            Label {
                Text(strings.text(WelcomeText.accountsNote))
                    .fixedSize(horizontal: false, vertical: true)
            } icon: {
                Image(systemName: "person.crop.circle.badge.checkmark")
            }
            .font(.callout)
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity, alignment: .leading)
        }

        private var buttons: some View {
            HStack {
                Button(strings.text(SettingsText.settingsMenu)) { flow.choose(.settings) }
                    .controlSize(.large)
                Spacer()
                Button(strings.text(WelcomeText.openHeadroom)) { flow.choose(.openHeadroom) }
                    .controlSize(.large)
                    .buttonStyle(.borderedProminent)
                    .keyboardShortcut(.defaultAction)
            }
        }
    }
#endif
