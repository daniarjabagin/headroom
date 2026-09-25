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

public struct FooterStatus: Sendable, Hashable {
    public let text: String
    public let isNotice: Bool
    public let refreshes: Bool

    public static func make(screen: PopupScreen, now: Timestamp, formatter: DisplayFormatter) -> FooterStatus {
        let strings = formatter.strings
        switch screen {
        case .serviceDown:
            return FooterStatus(text: strings.text(.serviceNotRunning), isNotice: false, refreshes: false)
        case .loading: return FooterStatus(text: strings.text(.connecting), isNotice: false, refreshes: false)
        case .incompatible, .unreadable: return FooterStatus(text: "", isNotice: false, refreshes: false)
        case .empty(let state), .dashboard(let state): return make(state: state, now: now, formatter: formatter)
        }
    }

    static func make(state: DaemonState, now: Timestamp, formatter: DisplayFormatter) -> FooterStatus {
        let strings = formatter.strings
        if state.offline {
            let text =
                state.lastSuccessAt.map { strings.fill(.offlineSince, ["time": formatter.clockTime($0.date)]) }
                ?? strings.text(.offline)
            return FooterStatus(text: text, isNotice: true, refreshes: true)
        }
        if PopupScreen.isRefreshing(state) {
            return FooterStatus(text: strings.text(.updating), isNotice: false, refreshes: true)
        }
        if let next = state.nextRefreshAt {
            return FooterStatus(text: formatter.nextUpdateText(next, now: now), isNotice: false, refreshes: true)
        }
        let updated = state.lastSuccessAt.map { strings.fill(.updatedAt, ["time": formatter.clockTime($0.date)]) }
        return FooterStatus(text: updated ?? "", isNotice: false, refreshes: true)
    }
}
