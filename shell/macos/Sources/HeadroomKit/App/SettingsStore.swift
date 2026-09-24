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

    @discardableResult
    public func change(_ change: SettingsChange) -> Task<CommandQueue.Outcome, Never> {
        if let settings { self.settings = change.applied(to: settings) }
        writesInFlight += 1
        let write = queue.enqueue(.updateSettings(change.patch))
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
