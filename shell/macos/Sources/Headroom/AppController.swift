#if canImport(AppKit)
    import AppKit
    import Foundation
    import HeadroomKit
    import HeadroomSettings
    import Observation
    import os

    @MainActor
    final class AppController {
        let model: AppModel
        let store: SettingsStore
        let environment: AppEnvironment
        var onOpenRequested: (@MainActor () -> Void)?

        private let client: DaemonClient
        private let alerts: AlertPoster?
        private let updater: SparkleUpdates?
        private let log = Logger(subsystem: "io.github.headroom", category: "app")
        private var supervisor: DaemonSupervisor?
        private var pumps: [Task<Void, Never>] = []
        private var helperEnvironment: [String: String]
        private var appliedTheme: ThemePreference?

        init(environment: AppEnvironment) {
            self.environment = environment
            client = DaemonClient(transport: UnixSocketTransport(path: environment.socketPath))
            let commands = CommandQueue(client: client)
            model = AppModel(
                preferredLanguages: Locale.preferredLanguages, appVersion: environment.bundledVersion,
                commands: commands)
            store = SettingsStore(client: client, queue: commands)
            alerts = Bundle.main.bundleIdentifier == nil ? nil : AlertPoster()
            updater = SparkleUpdates.makeIfConfigured()
            helperEnvironment = DaemonEnvironment.helper(
                daemon: environment.processEnvironment, socketPath: environment.socketPath)
        }

        var notificationAuthorizer: NotificationAuthorizer? { alerts?.authorizer }

        var updates: UpdatesModel? { updater?.model }

        var helperLauncher: HelperLauncher {
            HelperLauncher(executable: environment.helper, environment: helperEnvironment)
        }

        func start() async {
            alerts?.onActivate = { [weak self] in self?.onOpenRequested?() }
            alerts?.start()
            followTheme()
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
            helperEnvironment = DaemonEnvironment.helper(daemon: variables, socketPath: environment.socketPath)
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

        private func followTheme() {
            let theme = withObservationTracking {
                model.state?.display.theme
            } onChange: { [weak self] in
                Task { @MainActor in self?.followTheme() }
            }
            let wanted = theme ?? .system
            guard wanted != appliedTheme else { return }
            appliedTheme = wanted
            NSApp.appearance = wanted.appearance
        }

        private func handle(_ event: DaemonEvent) {
            model.apply(event)
            switch event {
            case .alert(let alert): alerts?.post(alert)
            case .openRequested: onOpenRequested?()
            case .connected: reloadSettings()
            case .failure(let error): log.error("daemon client: \(String(describing: error), privacy: .public)")
            default: return
            }
        }

        private func reloadSettings() {
            Task { [store] in await store.reload() }
        }

        private func handle(_ event: SupervisorEvent) {
            model.apply(event)
            log.notice("daemon supervisor: \(String(describing: event), privacy: .public)")
        }
    }
#endif
