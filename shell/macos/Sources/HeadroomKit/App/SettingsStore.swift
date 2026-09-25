import Foundation
import Observation

public enum ProviderList: Sendable, Hashable {
    case loading
    case loaded([ProviderInfo])
    case failed(String)
}

@MainActor
@Observable
public final class SettingsStore {
    public nonisolated static let providersSchemaVersion = 1

    public private(set) var settings: Settings?
    public private(set) var providers: ProviderList = .loading
    public private(set) var lastError: DaemonError?

    private let client: DaemonClient
    private let queue: CommandQueue
    private var writesInFlight = 0

    public init(client: DaemonClient, queue: CommandQueue) {
        self.client = client
        self.queue = queue
    }

    public func reload() async {
        await reloadSettings()
        await reloadProviders()
    }

    public var features: DaemonFeatures { settings?.features ?? .legacy }

    @discardableResult
    public func change(_ change: SettingsChange) -> Task<CommandQueue.Outcome, Never> {
        if let refusal = refusal(of: change) { return Task { .failure(refusal) } }
        if let settings { self.settings = change.applied(to: settings) }
        return track(queue.enqueue(.updateSettings(change.patch)))
    }

    @discardableResult
    public func resetAll() -> Task<CommandQueue.Outcome, Never> {
        guard features.release06 else { return Task { .failure(Self.needsRelease06) } }
        return track(queue.enqueue(.resetSettings))
    }

    public func diagnostics() async throws(DaemonError) -> Diagnostics {
        try await client.getDiagnostics()
    }

    nonisolated static let needsRelease06 = DaemonError.invalidArguments("This needs Headroom 0.6 or later")

    private func refusal(of change: SettingsChange) -> DaemonError? {
        guard features.allows(change) else { return Self.needsRelease06 }
        guard let settings, let problem = SettingsRules.problem(in: change.applied(to: settings)) else { return nil }
        return .invalidArguments("Invalid setting: \(problem)")
    }

    private func track(_ write: Task<CommandQueue.Outcome, Never>) -> Task<CommandQueue.Outcome, Never> {
        writesInFlight += 1
        return Task {
            let outcome = await write.value
            await self.settle(outcome)
            return outcome
        }
    }

    private func settle(_ outcome: CommandQueue.Outcome) async {
        writesInFlight -= 1
        if writesInFlight == 0 { await reloadSettings() }
        if case .failure(let error) = outcome { lastError = error }
    }

    private func reloadSettings() async {
        do {
            let loaded = try await client.getSettings()
            guard writesInFlight == 0 else { return }
            settings = loaded
            lastError = nil
        } catch {
            lastError = error
        }
    }

    private func reloadProviders() async {
        do {
            providers = Self.list(try await client.listProviders())
        } catch {
            providers = .failed(Self.describe(error))
        }
    }

    nonisolated static func list(_ payload: ProvidersPayload) -> ProviderList {
        guard payload.version == providersSchemaVersion else {
            return .failed("The Headroom service lists providers in version \(payload.version)")
        }
        return .loaded(payload.providers.filter { !$0.supportedMethods.isEmpty })
    }

    public nonisolated static func describe(_ error: DaemonError) -> String {
        switch error {
        case .invalidArguments(let message), .daemonFailure(let message), .rpc(_, let message): message
        default: String(describing: error)
        }
    }
}
