#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import SwiftUI

    struct RecoveryPlate: View {
        static let copiedDuration: Duration = .milliseconds(1500)

        let kind: NoticeKind
        let title: String
        let detail: String?
        let note: String?
        let recovery: NoticeRecovery?
        let context: PopupContext
        @State private var copied = false

        var body: some View {
            NoticePlate(kind: kind, title: title, detail: detail, note: note, actions: actions)
                .task(id: copied) {
                    guard copied else { return }
                    try? await Task.sleep(for: Self.copiedDuration)
                    guard !Task.isCancelled else { return }
                    copied = false
                }
        }

        private var actions: [NoticeAction] {
            guard let recovery else { return [] }
            return primaryActions(recovery.primary, accountID: recovery.accountID) + [retryAction(recovery)]
        }

        private func retryAction(_ recovery: NoticeRecovery) -> NoticeAction {
            let busy = context.retries.isBusy(recovery)
            let retries = context.retries
            let refresh = context.actions.refreshAccount
            let accountID = recovery.accountID
            return NoticeAction(
                id: "retry", title: context.strings.text(busy ? .retrying : .retry), primary: false, busy: busy,
                run: { retries.press(accountID, refresh) })
        }

        private func primaryActions(_ primary: RecoveryPrimary?, accountID: String) -> [NoticeAction] {
            switch primary {
            case .signIn(let provider):
                return [signInAction(.signInAgain, accountID: accountID, provider: provider)]
            case .cliSignIn(let provider, let command):
                return [
                    signInAction(.signIn, accountID: accountID, provider: provider),
                    copyAction(command, primary: false),
                ]
            case .copyCommand(let command):
                return [copyAction(command, primary: true)]
            case nil:
                return []
            }
        }

        private func signInAction(_ title: PopupText, accountID: String, provider: String) -> NoticeAction {
            let signInAgain = context.sections.signInAgain
            return NoticeAction(
                id: "signin", title: context.strings.text(title), primary: true, busy: false,
                run: { signInAgain(accountID, provider) })
        }

        private func copyAction(_ command: String, primary: Bool) -> NoticeAction {
            NoticeAction(
                id: "copy", title: context.strings.text(copied ? .copied : .copyCommand), primary: primary,
                busy: false, run: { copy(command) })
        }

        private func copy(_ command: String) {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(command, forType: .string)
            copied = true
        }
    }
#endif
