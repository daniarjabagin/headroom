public enum PopupScreen: Sendable, Hashable {
    case loading
    case incompatible(UIText)
    case serviceDown(detail: String?)
    case unreadable(message: String)
    case empty(DaemonState)
    case dashboard(DaemonState)

    public static func make(
        phase: ConnectionPhase, state: DaemonState?, lastError: DaemonError?, serviceIssue: String?
    ) -> PopupScreen {
        switch phase {
        case .incompatible(let text): return .incompatible(text)
        case .disconnected: return .serviceDown(detail: serviceIssue)
        case .starting, .connecting: return state.map(ready) ?? .loading
        case .connected:
            if let state { return ready(state) }
            return lastError.flatMap(unreadableMessage).map { .unreadable(message: $0) } ?? .loading
        }
    }

    public static func isRefreshing(_ state: DaemonState) -> Bool {
        state.accounts.contains { $0.status == .refreshing }
    }

    public static func refreshingIDs(_ state: DaemonState) -> Set<String> {
        Set(state.accounts.filter { $0.status == .refreshing }.map(\.id))
    }

    private static func ready(_ state: DaemonState) -> PopupScreen {
        let hasAccounts = state.accounts.contains { !$0.hidden }
        return hasAccounts || SpendCardModel.shows(state) ? .dashboard(state) : .empty(state)
    }

    private static func unreadableMessage(_ error: DaemonError) -> String? {
        switch error {
        case .invalidResponse(let message), .parseError(let message): message
        default: nil
        }
    }
}
