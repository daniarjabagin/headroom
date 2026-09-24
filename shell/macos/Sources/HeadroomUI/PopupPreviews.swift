#if canImport(AppKit) && DEBUG
    import HeadroomKit
    import SwiftUI

    @MainActor
    enum PreviewModels {
        static let actions = PopupActions(
            refreshNow: { true }, refreshAccount: { _ in }, openSettings: {}, signIn: { _ in },
            setAccountOrder: { _ in }, updateSettings: { _ in }, reducedMotion: { false })

        static func connected(_ state: DaemonState?) -> AppModel {
            let model = AppModel(preferredLanguages: ["en"])
            model.apply(.connected)
            if let state { model.apply(.state(state)) }
            return model
        }

        static func loading() -> AppModel {
            AppModel(preferredLanguages: ["en"])
        }

        static func serviceDown() -> AppModel {
            let model = AppModel(preferredLanguages: ["en"])
            model.apply(.exited(status: 1, restartIn: .seconds(4)))
            model.apply(.disconnected(nil))
            return model
        }

        static func incompatible() -> AppModel {
            let model = AppModel(preferredLanguages: ["en"])
            model.markHelperMismatch()
            return model
        }
    }

    #Preview("Dashboard") {
        PopupView(model: PreviewModels.connected(PreviewFixtures.state()), actions: PreviewModels.actions)
    }

    #Preview("Dashboard, dark") {
        PopupView(model: PreviewModels.connected(PreviewFixtures.state()), actions: PreviewModels.actions)
            .preferredColorScheme(.dark)
    }

    #Preview("Single provider") {
        PopupView(
            model: PreviewModels.connected(PreviewFixtures.state(singleProvider: true)), actions: PreviewModels.actions)
    }

    #Preview("Offline") {
        PopupView(model: PreviewModels.connected(PreviewFixtures.state(offline: true)), actions: PreviewModels.actions)
    }

    #Preview("Loading") {
        PopupView(model: PreviewModels.loading(), actions: PreviewModels.actions)
    }

    #Preview("No tools") {
        PopupView(model: PreviewModels.connected(PreviewFixtures.empty), actions: PreviewModels.actions)
    }

    #Preview("Service down") {
        PopupView(model: PreviewModels.serviceDown(), actions: PreviewModels.actions)
    }

    #Preview("Incompatible") {
        PopupView(model: PreviewModels.incompatible(), actions: PreviewModels.actions)
    }
#endif
