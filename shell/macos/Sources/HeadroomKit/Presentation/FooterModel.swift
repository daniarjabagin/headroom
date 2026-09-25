public enum FooterKind: Sendable, Hashable {
    case plain, live, stale
}

public struct FooterModel: Sendable, Hashable {
    public let primary: String
    public let secondary: String
    public let kind: FooterKind
    public let tip: String?
    public let refreshes: Bool

    public static func make(
        screen: PopupScreen, now: Timestamp, formatter: DisplayFormatter, version: String
    ) -> FooterModel {
        let strings = formatter.strings
        switch screen {
        case .serviceDown: return plain(version, strings.text(.serviceNotRunning))
        case .loading: return plain(version, strings.text(.connecting))
        case .incompatible, .unreadable: return plain(version, "")
        case .empty(let state), .dashboard(let state):
            return FooterStates(state: state, now: now, formatter: formatter, version: version).model
        }
    }

    static func plain(_ primary: String, _ secondary: String) -> FooterModel {
        FooterModel(primary: primary, secondary: secondary, kind: .plain, tip: nil, refreshes: false)
    }
}

struct FooterStates {
    let state: DaemonState
    let now: Timestamp
    let formatter: DisplayFormatter
    let version: String

    private var strings: UIStrings { formatter.strings }
    private var visible: [Account] { state.accounts.filter { !$0.hidden } }
    private var exact: Bool { state.display.resetFormat == .exact }

    var model: FooterModel {
        if state.offline { return offline }
        let stale = visible.filter { $0.status == .stale }
        if !stale.isEmpty { return outdated(stale) }
        if PopupScreen.isRefreshing(state) { return footer(updated, strings.text(.updating), .plain) }
        let live = visible.filter { $0.refresh?.mode == .live }
        if !live.isEmpty { return liveModel(live) }
        return footer(updated, nextUpdate, .plain)
    }

    private var updated: String {
        guard let last = state.lastSuccessAt else { return version }
        if exact { return strings.fill(.updatedAt, ["time": formatter.clockTime(last.date)]) }
        return strings.fill(PopupExtraText.updatedAgo, ["ago": formatter.agoText(last, now: now)])
    }

    private var nextUpdate: String {
        guard let next = state.nextRefreshAt else { return "" }
        if exact && next > now {
            return strings.fill(PopupExtraText.nextUpdateAt, ["time": formatter.clockTime(next.date)])
        }
        return formatter.nextUpdateText(next, now: now)
    }

    private var offline: FooterModel {
        let retry = state.nextRefreshAt.map { next in
            let left = max(1, next.seconds(since: now))
            return strings.fill(
                PopupExtraText.offlineRetrying, ["duration": formatter.duration(seconds: left, withSeconds: left < 60)])
        }
        return footer(outdatedLine(state.lastSuccessAt), retry ?? strings.text(.offline), .stale)
    }

    private func outdated(_ stale: [Account]) -> FooterModel {
        let oldest = stale.compactMap(\.updatedAt).min()
        return footer(outdatedLine(oldest), nextUpdate, .stale)
    }

    private func outdatedLine(_ updatedAt: Timestamp?) -> String {
        guard let updatedAt else { return strings.text(.outdated) }
        return strings.fill(PopupExtraText.outdatedUpdated, ["ago": formatter.agoText(updatedAt, now: now)])
    }

    private func liveModel(_ live: [Account]) -> FooterModel {
        var seen: Set<String> = []
        let names = live.map(\.providerName).filter { seen.insert($0).inserted }.joined(separator: ", ")
        let seconds = live.map(\.refresh).compactMap { $0?.intervalSecs }.min() ?? 60
        let interval = lowered(strings.refreshInterval(seconds: seconds))
        let line = strings.fill(PopupExtraText.liveEvery, ["interval": interval, "providers": names])
        let tip = strings.fill(PopupExtraText.liveTip, ["providers": names, "interval": interval])
        let idle = visible.compactMap(\.refresh).first { $0.mode == .idle }.map { refresh in
            strings.fill(
                PopupExtraText.liveTipInterval,
                ["interval": lowered(strings.refreshInterval(seconds: refresh.intervalSecs))])
        }
        return FooterModel(
            primary: updated, secondary: line, kind: .live,
            tip: [tip, idle].compactMap { $0 }.joined(separator: " "), refreshes: true)
    }

    private func footer(_ primary: String, _ secondary: String, _ kind: FooterKind) -> FooterModel {
        FooterModel(primary: primary, secondary: secondary, kind: kind, tip: nil, refreshes: true)
    }

    private func lowered(_ text: String) -> String {
        guard let first = text.first else { return text }
        return first.lowercased() + text.dropFirst()
    }
}
