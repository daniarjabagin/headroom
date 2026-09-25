#if canImport(AppKit)
    import Foundation
    import HeadroomKit
    import Observation

    @MainActor
    @Observable
    final class RetryControl {
        private var tracker = RetryTracker()
        private var now = Date()
        @ObservationIgnored private var timer: Task<Void, Never>?

        func isBusy(_ recovery: NoticeRecovery) -> Bool {
            recovery.retrying || tracker.isPending(recovery.accountID, at: now)
        }

        func press(_ accountID: String, _ refresh: @escaping @MainActor (String) async -> Bool) {
            guard tracker.press(accountID, at: Date()) else { return }
            sync()
            Task { [weak self] in
                let succeeded = await refresh(accountID)
                self?.settle(accountID, succeeded: succeeded)
            }
        }

        func observe(refreshing ids: Set<String>) {
            tracker.observe(refreshing: ids)
            sync()
        }

        private func settle(_ accountID: String, succeeded: Bool) {
            tracker.settle(accountID, succeeded: succeeded)
            sync()
        }

        private func sync() {
            let current = Date()
            now = current
            timer?.cancel()
            guard let deadline = tracker.nextExpiry(after: current) else { return }
            timer = Task { [weak self] in
                try? await Task.sleep(for: .seconds(deadline.timeIntervalSince(current)))
                guard !Task.isCancelled else { return }
                self?.sync()
            }
        }
    }
#endif
