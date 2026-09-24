#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import SwiftUI

    struct WindowRow: View {
        let window: QuotaWindow
        let display: DisplaySettings
        let formatter: DisplayFormatter
        let now: Timestamp

        private var percent: Double { DisplayFormatter.readingPercent(window, mode: display.valueMode) }

        private var tick: Double? {
            guard let even = window.pace.evenPacePercent else { return nil }
            return display.valueMode == .used ? even : 100 - even
        }

        var body: some View {
            VStack(alignment: .leading, spacing: 4) {
                Text(formatter.windowLabel(id: window.id, label: window.label))
                    .font(.system(size: 13, weight: .semibold))
                PillMeter(fraction: percent / 100, tick: tick.map { $0 / 100 }, tone: window.tone)
                HStack {
                    Text(formatter.percentReading(percent, mode: display.valueMode))
                    Spacer(minLength: 8)
                    Text(formatter.resetText(resetsAt: window.resetsAt, now: now, format: display.resetFormat))
                        .foregroundStyle(.secondary)
                }
                .font(.system(size: 12))
                .monospacedDigit()
            }
            .padding(.horizontal, PopupMetrics.padding)
            .padding(.vertical, 10)
        }
    }

    struct PillMeter: View {
        let fraction: Double
        let tick: Double?
        let tone: Tone

        var body: some View {
            GeometryReader { proxy in
                let width = proxy.size.width
                ZStack(alignment: .leading) {
                    Capsule().fill(Color.primary.opacity(0.1))
                    Capsule()
                        .fill(tone.color)
                        .frame(width: fillWidth(in: width))
                    if let tick {
                        RoundedRectangle(cornerRadius: 1)
                            .fill(Color.primary.opacity(0.55))
                            .frame(width: PopupMetrics.tickWidth, height: PopupMetrics.tickHeight)
                            .offset(x: tickOffset(tick, in: width))
                    }
                }
            }
            .frame(height: PopupMetrics.meterHeight)
        }

        private func fillWidth(in width: CGFloat) -> CGFloat {
            let clamped = min(1, max(0, fraction))
            return clamped == 0 ? 0 : max(PopupMetrics.meterHeight, width * clamped)
        }

        private func tickOffset(_ tick: Double, in width: CGFloat) -> CGFloat {
            let usable = max(0, width - PopupMetrics.tickWidth)
            return usable * min(1, max(0, tick))
        }
    }
#endif
