import Foundation

public enum StatusText: LocalizedText {
    case incident, partialOutage, majorOutage, maintenance, degraded
    case investigating, identified, monitoring, inProgress, verifying
    case startedAgo, startedAt, elapsed

    public var translations: (english: String, russian: String) {
        switch self {
        case .incident: ("Incident", "Инцидент")
        case .partialOutage: ("Partial outage", "Частичный сбой")
        case .majorOutage: ("Major outage", "Серьёзный сбой")
        case .maintenance: ("Maintenance", "Техработы")
        case .degraded: ("Degraded performance", "Снижена производительность")
        case .investigating: ("Investigating", "Выясняют причину")
        case .identified: ("Identified — a fix is on the way", "Причина найдена — готовят исправление")
        case .monitoring: ("Monitoring — a fix is in place", "Исправление выпущено, наблюдают")
        case .inProgress: ("In progress", "Идут работы")
        case .verifying: ("Verifying", "Проверяют")
        case .startedAgo: ("Started {ago}", "Началось {ago}")
        case .startedAt: ("Started at {time}", "Началось в {time}")
        case .elapsed: ("· {duration}", "· {duration}")
        }
    }
}

public struct StatusNoticeModel: Sendable, Hashable, Identifiable {
    public let id: String
    public let kind: NoticeKind
    public let heading: String
    public let title: String
    public let detail: String?
    public let started: String?
    public let startedTip: String?
    public let elapsed: String?
    public let url: URL?
    public let linkTitle: String

    public static func make(
        provider: String, statuses: [ProviderStatus], now: Timestamp, formatter: DisplayFormatter
    ) -> StatusNoticeModel? {
        guard let status = statuses.first(where: { $0.provider == provider }), shows(status) else { return nil }
        let strings = formatter.strings
        let heading = heading(status, strings: strings)
        return StatusNoticeModel(
            id: "status:\(provider)", kind: status.tone == .critical ? .error : .warning, heading: heading,
            title: status.title.map { "\(heading) · \($0)" } ?? heading,
            detail: status.stage.map { stage($0, strings: strings) },
            started: status.startedAt.map {
                strings.fill(StatusText.startedAgo, ["ago": formatter.agoText($0, now: now)])
            },
            startedTip: status.startedAt.map {
                strings.fill(StatusText.startedAt, ["time": formatter.clockTime($0.date)])
            },
            elapsed: status.startedAt.map {
                strings.fill(StatusText.elapsed, ["duration": formatter.duration(seconds: now.seconds(since: $0))])
            },
            url: secureURL(status.url), linkTitle: strings.text(PopupExtraText.statusPage))
    }

    static func shows(_ status: ProviderStatus) -> Bool {
        !status.isClear && (status.tone == .warning || status.tone == .critical)
    }

    static func heading(_ status: ProviderStatus, strings: UIStrings) -> String {
        switch status.indicator {
        case .critical: strings.text(StatusText.majorOutage)
        case .major: strings.text(StatusText.partialOutage)
        case .maintenance: strings.text(StatusText.maintenance)
        case .minor, .none, .unknown:
            strings.text(status.title == nil ? StatusText.degraded : .incident)
        }
    }

    static func stage(_ stage: String, strings: UIStrings) -> String {
        switch stage {
        case "investigating": strings.text(StatusText.investigating)
        case "identified": strings.text(StatusText.identified)
        case "monitoring": strings.text(StatusText.monitoring)
        case "in_progress": strings.text(StatusText.inProgress)
        case "verifying": strings.text(StatusText.verifying)
        default: stage.prefix(1).uppercased() + stage.dropFirst().replacingOccurrences(of: "_", with: " ")
        }
    }

    static func secureURL(_ text: String) -> URL? {
        guard text.hasPrefix("https://"), let url = URL(string: text), url.host?.isEmpty == false else { return nil }
        return url
    }
}
