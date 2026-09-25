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
            return [primaryAction(recovery.primary), retryAction(recovery)].compactMap { $0 }
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

        private func primaryAction(_ primary: RecoveryPrimary?) -> NoticeAction? {
            switch primary {
            case .signIn(let provider):
                let signIn = context.actions.signIn
                return NoticeAction(
                    id: "signin", title: context.strings.text(.signInAgain), primary: true, busy: false,
                    run: { signIn(provider) })
            case .copyCommand(let command):
                return NoticeAction(
                    id: "copy", title: context.strings.text(copied ? .copied : .copyCommand), primary: true,
                    busy: false, run: { copy(command) })
            case nil:
                return nil
            }
        }

        private func copy(_ command: String) {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(command, forType: .string)
            copied = true
        }
    }
#endif
