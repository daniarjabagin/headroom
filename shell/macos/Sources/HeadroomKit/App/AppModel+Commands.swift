public enum SettingsRoute: Sendable, Hashable {
    case general
    case accounts
    case addAccount(provider: String?)
    case signInAgain(accountID: String, provider: String)
    case notifications
    case service
}

struct PendingOrder: Sendable, Hashable {
    let accountIDs: [String]
    var settled = false

    static func arrange(_ accounts: [Account], by order: [String]) -> [Account] {
        let rank = Dictionary(order.enumerated().map { ($1, $0) }, uniquingKeysWith: { first, _ in first })
        return accounts.enumerated()
            .sorted { lhs, rhs in
                (rank[lhs.element.id] ?? order.count + lhs.offset) < (rank[rhs.element.id] ?? order.count + rhs.offset)
            }
            .map(\.element)
    }
}

extension AppModel {
    public func refreshNow() {
        send(.refreshNow)
    }

    public func refresh(accountID: String) async -> Bool {
        guard let task = send(.refresh(accountID: accountID)) else { return false }
        if case .success = await task.value { return true }
        return false
    }

    public func setAccountOrder(_ accountIDs: [String]) {
        pendingOrder = PendingOrder(accountIDs: accountIDs)
        let task = send(.setAccountOrder(accountIDs))
        Task {
            let outcome = await task?.value
            guard pendingOrder?.accountIDs == accountIDs else { return }
            if case .some(.success) = outcome { pendingOrder?.settled = true } else { pendingOrder = nil }
        }
    }

    public func openSettings() {
        settingsPresenter?(.general)
    }

    public func signIn(provider: String) {
        settingsPresenter?(.addAccount(provider: provider))
    }

    public func signInAgain(accountID: String, provider: String) {
        settingsPresenter?(.signInAgain(accountID: accountID, provider: provider))
    }

    @discardableResult
    public func send(_ command: DaemonCommand) -> Task<CommandQueue.Outcome, Never>? {
        guard let commands else { return nil }
        let task = commands.enqueue(command)
        Task {
            if case .failure(let error) = await task.value { record(error) }
        }
        return task
    }
}
