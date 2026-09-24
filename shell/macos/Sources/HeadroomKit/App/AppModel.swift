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

    private let preferredLanguages: [String]
    private let timeZone: TimeZone

    public init(preferredLanguages: [String], timeZone: TimeZone = .current) {
        self.preferredLanguages = preferredLanguages
        self.timeZone = timeZone
    }

    public var formatter: DisplayFormatter {
        let preference = state?.display.language ?? .system
        let language = UILanguage.resolve(preference, preferredLanguages: preferredLanguages)
        return DisplayFormatter(language: language, timeZone: timeZone)
    }

    public var menuBarContent: MenuBarContent {
        guard phase == .connected else { return .glyph }
        return MenuBarContent.make(state: state, formatter: formatter)
    }

    public var visibleAccounts: [Account] {
        state?.accounts.filter { !$0.hidden } ?? []
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
            state = newState
            phase = .connected
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

    public func markHelperMismatch() {
        phase = .incompatible(.helperMismatch)
    }

    private func record(_ error: DaemonError) {
        lastError = error
        if case .unsupportedSchema = error { phase = .incompatible(.schemaMismatch) }
    }
}
