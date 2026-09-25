import Foundation
import Observation

public enum ConnectionPhase: Sendable, Hashable {
    case starting
    case connecting
    case connected
    case disconnected
    case incompatible(UIText)
}

@MainActor
@Observable
public final class AppModel {
    public private(set) var state: DaemonState?
    public private(set) var phase: ConnectionPhase = .starting
    public private(set) var lastError: DaemonError?
    public private(set) var serviceIssue: String?
    public var settingsPresenter: (@MainActor (SettingsRoute) -> Void)?

    let commands: CommandQueue?
    var pendingOrder: PendingOrder?
    private let preferredLanguages: [String]
    private let timeZone: TimeZone
    private let locale: Locale
    private let appVersion: String?

    public init(
        preferredLanguages: [String], timeZone: TimeZone = .current, locale: Locale = .current,
        appVersion: String? = nil, commands: CommandQueue? = nil
    ) {
        self.preferredLanguages = preferredLanguages
        self.timeZone = timeZone
        self.locale = locale
        self.appVersion = appVersion
        self.commands = commands
    }

    public var formatter: DisplayFormatter {
        let preference = state?.display.language ?? .system
        let language = UILanguage.resolve(preference, preferredLanguages: preferredLanguages)
        let hourCycle = HourCycle.resolve(state?.display.timeFormat ?? .auto, locale: locale)
        return DisplayFormatter(language: language, timeZone: timeZone, hourCycle: hourCycle)
    }

    public var features: DaemonFeatures { state?.features ?? .legacy }

    public var menuBarContent: MenuBarContent {
        guard phase == .connected else { return .glyph }
        return MenuBarContent.make(state: state, formatter: formatter)
    }

    public var orderedAccounts: [Account] {
        let accounts = state?.accounts ?? []
        guard let order = pendingOrder?.accountIDs else { return accounts }
        return PendingOrder.arrange(accounts, by: order)
    }

    public var visibleAccounts: [Account] {
        orderedAccounts.filter { !$0.hidden }
    }

    public func apply(_ event: DaemonEvent) {
        switch event {
        case .connecting:
            if phase == .starting { phase = .connecting }
        case .connected:
            if case .incompatible = phase { return }
            phase = .connected
            lastError = nil
        case .disconnected(let error):
            if case .incompatible = phase { return }
            phase = .disconnected
            lastError = error
        case .state(let newState):
            receive(newState)
        case .failure(let error):
            record(error)
        case .alert, .openRequested:
            return
        }
    }

    public func apply(_ event: SupervisorEvent) {
        switch event {
        case .started:
            serviceIssue = nil
        case .exited(let status, let delay):
            serviceIssue = "headroom daemon exited with status \(status), restarting in \(delay)"
        case .launchFailed(let message, let delay):
            serviceIssue = "\(message), retrying in \(delay)"
        }
    }

    private func isServiceCompatible(_ candidate: DaemonState) -> Bool {
        guard let appVersion, let serviceVersion = candidate.appVersion else { return true }
        return serviceVersion == appVersion
    }

    private func receive(_ newState: DaemonState) {
        guard isServiceCompatible(newState) else {
            state = nil
            phase = .incompatible(.differentService)
            return
        }
        state = newState
        phase = .connected
        if pendingOrder?.settled == true { pendingOrder = nil }
    }

    public func markHelperMismatch() {
        phase = .incompatible(.helperMismatch)
    }

    func record(_ error: DaemonError) {
        lastError = error
        if case .unsupportedSchema = error { phase = .incompatible(.schemaMismatch) }
    }
}
