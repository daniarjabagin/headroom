#if canImport(AppKit)
    import Foundation
    import HeadroomKit
    import Observation
    import SwiftUI

    @MainActor
    @Observable
    final class RefreshControl {
        private(set) var mode = RefreshMode.idle
        private(set) var spin = SpinMotion()
        private(set) var shakeStartedAt: Date?
        private(set) var animating = false
        @ObservationIgnored private var tracker = RefreshTracker()
        @ObservationIgnored private var timer: Task<Void, Never>?

        func press(_ refresh: @escaping @MainActor () async -> Bool) {
            guard tracker.press(at: Date()) else { return }
            sync()
            Task { [weak self] in
                let succeeded = await refresh()
                self?.settle(succeeded)
            }
        }

        func setDaemonBusy(_ busy: Bool) {
            tracker.setDaemonBusy(busy)
            sync()
        }

        private func settle(_ succeeded: Bool) {
            tracker.settle(succeeded: succeeded, at: Date())
            sync()
        }

        private func sync() {
            let now = Date()
            let next = tracker.mode(at: now)
            if next != mode { transition(to: next, at: now) }
            let shaking = shakeStartedAt.map { now.timeIntervalSince($0) < ShakeMotion.duration } ?? false
            animating = spin.isAnimating(at: now) || shaking
            schedule(after: now)
        }

        private func transition(to next: RefreshMode, at now: Date) {
            if next == .busy { spin.start(at: now) } else { spin.stop(at: now) }
            if next == .failed { shakeStartedAt = now }
            mode = next
        }

        private func schedule(after now: Date) {
            timer?.cancel()
            let shakeEnd = shakeStartedAt.map { $0.addingTimeInterval(ShakeMotion.duration) }
            let deadlines = [tracker.nextChange(after: now), spin.restsAt(), shakeEnd].compactMap { $0 }
            guard let deadline = deadlines.filter({ $0 > now }).min() else { return }
            timer = Task { [weak self] in
                try? await Task.sleep(for: .seconds(deadline.timeIntervalSince(now)))
                guard !Task.isCancelled else { return }
                self?.sync()
            }
        }
    }

    struct RefreshButton: View {
        let control: RefreshControl
        let strings: UIStrings
        let press: @MainActor () -> Void
        @Environment(\.headroomReducedMotion) private var reducedMotion

        var body: some View {
            Button {
                press()
            } label: {
                TimelineView(.animation(minimumInterval: nil, paused: reducedMotion || !control.animating)) { context in
                    icon(at: context.date)
                }
                .frame(width: PopupMetrics.refreshSize, height: PopupMetrics.refreshSize)
            }
            .buttonStyle(TintButtonStyle(circle: true))
            .hoverTip(id: "refresh", text: tooltip)
            .accessibilityLabel(strings.text(.refresh))
        }

        private func icon(at date: Date) -> some View {
            Image(systemName: "arrow.clockwise")
                .font(.system(size: 14, weight: .medium))
                .foregroundStyle(color)
                .rotationEffect(.degrees(reducedMotion ? 0 : control.spin.angle(at: date)))
                .opacity(reducedMotion && control.mode == .busy ? 0.55 : 1)
                .offset(x: reducedMotion ? 0 : shakeOffset(at: date))
        }

        private var tooltip: String {
            control.mode == .failed ? strings.text(.refreshFailed) : strings.text(.refresh)
        }

        private var color: Color {
            switch control.mode {
            case .idle: Color.secondary
            case .busy: Palette.ok
            case .failed: Palette.crit
            }
        }

        private func shakeOffset(at date: Date) -> CGFloat {
            guard let start = control.shakeStartedAt else { return 0 }
            return CGFloat(ShakeMotion.offset(elapsed: date.timeIntervalSince(start)))
        }
    }
#endif
