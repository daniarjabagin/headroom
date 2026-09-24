#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import SwiftUI

    public struct PopupActions {
        public let refresh: @MainActor () -> Void
        public let quit: @MainActor () -> Void

        public init(refresh: @escaping @MainActor () -> Void, quit: @escaping @MainActor () -> Void) {
            self.refresh = refresh
            self.quit = quit
        }
    }

    public struct PopupView: View {
        let model: AppModel
        let actions: PopupActions

        public init(model: AppModel, actions: PopupActions) {
            self.model = model
            self.actions = actions
        }

        public var body: some View {
            VStack(alignment: .leading, spacing: PopupMetrics.sectionGap) {
                content
                PopupFooter(strings: model.formatter.strings, actions: actions)
            }
            .padding(PopupMetrics.padding)
            .frame(width: PopupMetrics.width, alignment: .leading)
            .popupSurface(translucent: model.state?.display.translucent ?? false)
        }

        @ViewBuilder
        private var content: some View {
            let strings = model.formatter.strings
            switch model.phase {
            case .incompatible(let text):
                StatusMessage(title: strings.text(text), detail: nil)
            case .connected:
                if let state = model.state {
                    AccountList(state: state, accounts: model.visibleAccounts, formatter: model.formatter)
                } else {
                    StatusMessage(title: strings.text(.connecting), detail: nil)
                }
            case .disconnected:
                StatusMessage(title: strings.text(.serviceNotRunning), detail: model.serviceIssue)
            case .starting, .connecting:
                StatusMessage(title: strings.text(.connecting), detail: nil)
            }
        }
    }

    struct StatusMessage: View {
        let title: String
        let detail: String?

        var body: some View {
            VStack(spacing: 6) {
                Text(title).font(.system(size: 13, weight: .semibold))
                if let detail {
                    Text(detail).font(.system(size: 11)).foregroundStyle(.secondary)
                }
            }
            .multilineTextAlignment(.center)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 24)
        }
    }

    struct PopupFooter: View {
        let strings: UIStrings
        let actions: PopupActions

        var body: some View {
            ActionGroup {
                Button(strings.text(.refresh)) { actions.refresh() }
                    .actionButtonStyle()
                Spacer(minLength: 0)
                Button(strings.text(.quit)) { actions.quit() }
                    .actionButtonStyle()
            }
            .font(.system(size: 12))
        }
    }
#endif
