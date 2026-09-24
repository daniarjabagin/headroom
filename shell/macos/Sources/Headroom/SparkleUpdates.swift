#if canImport(AppKit) && canImport(Sparkle)
    import AppKit
    import Foundation
    import HeadroomKit
    import Sparkle

    @MainActor
    final class SparkleUpdates: NSObject, UpdateControls {
        let model: UpdatesModel
        private let controller: SPUStandardUpdaterController
        private var observations: [NSKeyValueObservation] = []

        static func makeIfConfigured(bundle: Bundle = .main) -> SparkleUpdates? {
            let info = bundle.infoDictionary ?? [:]
            guard info["SUFeedURL"] is String, info["SUPublicEDKey"] is String else { return nil }
            let controller = SPUStandardUpdaterController(
                startingUpdater: true, updaterDelegate: nil, userDriverDelegate: nil)
            return SparkleUpdates(controller: controller)
        }

        private init(controller: SPUStandardUpdaterController) {
            self.controller = controller
            model = UpdatesModel()
            super.init()
            observeUpdater()
            model.connect(self)
        }

        var status: UpdaterStatus {
            let updater = controller.updater
            return UpdaterStatus(
                automaticallyChecks: updater.automaticallyChecksForUpdates,
                canCheck: updater.canCheckForUpdates,
                lastCheck: updater.lastUpdateCheckDate)
        }

        func setAutomaticallyChecks(_ enabled: Bool) {
            controller.updater.automaticallyChecksForUpdates = enabled
        }

        func checkNow() {
            controller.checkForUpdates(nil)
        }

        private func observeUpdater() {
            let updater = controller.updater
            observations = [
                updater.observe(\.canCheckForUpdates) { [weak self] _, _ in self?.scheduleRefresh() },
                updater.observe(\.automaticallyChecksForUpdates) { [weak self] _, _ in self?.scheduleRefresh() },
            ]
        }

        private nonisolated func scheduleRefresh() {
            Task { @MainActor [weak self] in self?.model.refresh() }
        }
    }
#endif
