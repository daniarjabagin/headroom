#if canImport(AppKit)
    import Foundation
    import HeadroomKit
    import os

    @MainActor
    final class AppController {
        let model: AppModel
        var onOpenRequested: (@MainActor () -> Void)?

        private let environment: AppEnvironment
        private let client: DaemonClient
        private let alerts: AlertPoster?
        private let log = Logger(subsystem: "io.github.headroom", category: "app")
        private var supervisor: DaemonSupervisor?
        private var pumps: [Task<Void, Never>] = []

        init(environment: AppEnvironment) {
            self.environment = environment
            model = AppModel(preferredLanguages: Locale.preferredLanguages)
            client = DaemonClient(transport: UnixSocketTransport(path: environment.socketPath))
            alerts = Bundle.main.bundleIdentifier == nil ? nil : AlertPoster()
        }

        func start() async {
            alerts?.onActivate = { [weak self] in self?.onOpenRequested?() }
            alerts?.start()
            if let helper = environment.helper {
                guard await helperMatchesApp(helper) else { return }
                await startSupervisor(helper: helper)
            }
            pumps.append(
                Task { [client] in
                    for await event in client.events { self.handle(event) }
                })
            await client.start()
        }

        func shutdown() async {
            pumps.forEach { $0.cancel() }
            await client.stop()
            await supervisor?.stop()
        }

        func refreshNow() {
            Task { [client] in
                do {
                    try await client.refreshNow()
                } catch {
                    self.handle(.failure(DaemonError.wrapping(error)))
                }
            }
        }

        func refreshIfDue() {
            Task { [client] in try? await client.refresh() }
        }

        private func helperMatchesApp(_ helper: URL) async -> Bool {
            guard let expected = environment.bundledVersion else { return true }
            let actual = try? await HelperVersion.query(helper: helper)
            guard actual == expected else {
                let found = actual ?? "unknown"
                log.error("helper \(found, privacy: .public) differs from app \(expected, privacy: .public)")
                model.markHelperMismatch()
                return false
            }
            return true
        }

        private func startSupervisor(helper: URL) async {
            let login = await LoginShellEnvironment.capture(shell: environment.loginShell)
            let variables = DaemonEnvironment.build(
                base: environment.processEnvironment, loginShell: login,
                preferredLanguage: Locale.preferredLanguages.first)
            let spec = LaunchSpec.daemon(
                helper: helper, socketPath: environment.socketPath, environment: variables,
                logFile: environment.logFile)
            let supervisor = DaemonSupervisor(spec: spec)
            self.supervisor = supervisor
            pumps.append(
                Task {
                    for await event in supervisor.events { self.handle(event) }
                })
            await supervisor.start()
        }

        private func handle(_ event: DaemonEvent) {
            model.apply(event)
            switch event {
            case .alert(let alert): alerts?.post(alert)
            case .openRequested: onOpenRequested?()
            case .failure(let error): log.error("daemon client: \(String(describing: error), privacy: .public)")
            default: return
            }
        }

        private func handle(_ event: SupervisorEvent) {
            model.apply(event)
            log.notice("daemon supervisor: \(String(describing: event), privacy: .public)")
        }
    }
#endif
