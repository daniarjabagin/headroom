#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct SheenClock: Equatable, Sendable {
        static let stopped = SheenClock(epoch: .distantPast, running: false)

        let epoch: Date
        let running: Bool
    }

    struct SheenClockKey: EnvironmentKey {
        static let defaultValue = SheenClock.stopped
    }

    extension EnvironmentValues {
        var sheenClock: SheenClock {
            get { self[SheenClockKey.self] }
            set { self[SheenClockKey.self] = newValue }
        }
    }

    struct SheenSchedule: TimelineSchedule {
        let epoch: Date

        func entries(from startDate: Date, mode: TimelineScheduleMode) -> UnfoldFirstSequence<Date> {
            let origin = epoch
            let frame = mode == .lowFrequency ? 0 : MeterSheen.frameSeconds
            return sequence(first: startDate) { date in
                let next = MeterSheen.nextFrame(after: date.timeIntervalSince(origin), frameSeconds: frame)
                return origin.addingTimeInterval(next)
            }
        }
    }

    struct MeterSheenModifier: ViewModifier {
        let fill: Double
        let width: CGFloat
        let height: CGFloat
        @Environment(\.sheenClock) private var clock
        @Environment(\.headroomReducedMotion) private var reducedMotion

        func body(content: Content) -> some View {
            content.overlay(alignment: .leading) {
                if clock.running && MeterSheen.runs(fill: fill, reducedMotion: reducedMotion) {
                    MeterSheenBand(epoch: clock.epoch, width: width, height: height)
                }
            }
        }
    }

    struct MeterSheenBand: View {
        let epoch: Date
        let width: CGFloat
        let height: CGFloat

        var body: some View {
            TimelineView(SheenSchedule(epoch: epoch)) { timeline in
                band(progress: MeterSheen.progress(elapsed: timeline.date.timeIntervalSince(epoch)))
            }
            .frame(width: width, height: height, alignment: .leading)
            .clipShape(Capsule())
            .allowsHitTesting(false)
            .accessibilityHidden(true)
        }

        @ViewBuilder
        private func band(progress: Double?) -> some View {
            if let progress {
                LinearGradient(
                    colors: [.white.opacity(0), .white.opacity(MeterSheen.peakOpacity), .white.opacity(0)],
                    startPoint: .leading, endPoint: .trailing
                )
                .frame(width: CGFloat(MeterSheen.bandWidth), height: height)
                .offset(x: CGFloat(MeterSheen.bandOffset(progress: progress, fillWidth: Double(width))))
                .frame(width: width, height: height, alignment: .leading)
            }
        }
    }

    extension View {
        func meterSheen(fill: Double, width: CGFloat, height: CGFloat) -> some View {
            modifier(MeterSheenModifier(fill: fill, width: width, height: height))
        }
    }
#endif
