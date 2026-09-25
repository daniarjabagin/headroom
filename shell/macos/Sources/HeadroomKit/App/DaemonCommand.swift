public enum DaemonCommand: Sendable, Hashable {
    case refreshNow
    case refresh(accountID: String)
    case setAccountOrder([String])
    case setAccountLabel(accountID: String, label: String)
    case setAccountHidden(accountID: String, hidden: Bool)
    case updateSettings([String: JSONValue])
    case resetSettings
    case restoreAccounts(provider: String)

    func perform(on client: DaemonClient) async throws(DaemonError) {
        switch self {
        case .refreshNow: try await client.refreshNow()
        case .refresh(let accountID): try await client.refresh(accountID: accountID)
        case .setAccountOrder(let accountIDs): try await client.setAccountOrder(accountIDs)
        case .setAccountLabel(let accountID, let label):
            try await client.setAccountLabel(accountID: accountID, label: label)
        case .setAccountHidden(let accountID, let hidden):
            try await client.setAccountHidden(accountID: accountID, hidden: hidden)
        case .updateSettings(let patch): try await client.updateSettings(patch)
        case .resetSettings: try await client.resetSettings()
        case .restoreAccounts(let provider): try await client.restoreAccounts(provider: provider)
        }
    }
}

@MainActor
public final class CommandQueue {
    public typealias Outcome = Result<Void, DaemonError>

    private let client: DaemonClient
    private var tail: Task<Void, Never>?

    public init(client: DaemonClient) {
        self.client = client
    }

    @discardableResult
    public func enqueue(_ command: DaemonCommand) -> Task<Outcome, Never> {
        let previous = tail
        let task = Task { [client] () -> Outcome in
            await previous?.value
            do throws(DaemonError) {
                try await command.perform(on: client)
                return .success(())
            } catch {
                return .failure(error)
            }
        }
        tail = Task { _ = await task.value }
        return task
    }
}
