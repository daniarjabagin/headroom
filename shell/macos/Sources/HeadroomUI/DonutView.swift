#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct AnimatableVector: VectorArithmetic, Sendable {
        var values: [Double]

        static var zero: AnimatableVector { AnimatableVector(values: []) }

        static func + (lhs: AnimatableVector, rhs: AnimatableVector) -> AnimatableVector {
            combine(lhs, rhs, +)
        }

        static func - (lhs: AnimatableVector, rhs: AnimatableVector) -> AnimatableVector {
            combine(lhs, rhs, -)
        }

        mutating func scale(by rhs: Double) {
            values = values.map { $0 * rhs }
        }

        var magnitudeSquared: Double {
            values.reduce(0) { $0 + $1 * $1 }
        }

        private static func combine(
            _ lhs: AnimatableVector, _ rhs: AnimatableVector, _ operation: (Double, Double) -> Double
        ) -> AnimatableVector {
            let count = max(lhs.values.count, rhs.values.count)
            return AnimatableVector(
                values: (0..<count).map { index in
                    operation(value(lhs, index), value(rhs, index))
                })
        }

        private static func value(_ vector: AnimatableVector, _ index: Int) -> Double {
            vector.values.indices.contains(index) ? vector.values[index] : 0
        }
    }

    struct DonutSliceShape: Shape {
        var fractions: AnimatableVector
        var reveal: Double
        let index: Int

        var animatableData: AnimatablePair<Double, AnimatableVector> {
            get { AnimatablePair(reveal, fractions) }
            set {
                reveal = newValue.first
                fractions = newValue.second
            }
        }

        func path(in rect: CGRect) -> Path {
            let size = min(rect.width, rect.height)
            let geometry = DonutGeometry(size: size)
            let originX = rect.midX - size / 2
            let originY = rect.midY - size / 2
            var path = Path()
            let segments = DonutGeometry.segments(fractions.values, reveal: reveal)
            guard let segment = segments.first(where: { $0.index == index }),
                let outline = geometry.outline(segment)
            else { return path }
            for loop in outline {
                path.addLines(loop.map { CGPoint(x: originX + $0.x, y: originY + $0.y) })
                path.closeSubpath()
            }
            return path
        }
    }

    struct DonutView: View {
        let model: SpendCardModel
        @State private var reveal = 0.0
        @Environment(\.headroomReducedMotion) private var reducedMotion

        var body: some View {
            ZStack {
                ForEach(Array(model.entries.enumerated()), id: \.element.id) { index, entry in
                    DonutSliceShape(fractions: AnimatableVector(values: model.fractions), reveal: reveal, index: index)
                        .fill(entry.color.color, style: FillStyle(eoFill: true, antialiased: true))
                }
                Text(model.centerAmount)
                    .font(Typeface.ring)
                    .monospacedDigit()
                    .lineLimit(1)
                    .minimumScaleFactor(0.6)
                    .padding(.horizontal, PopupMetrics.donutSize * 0.2)
                    .contentTransition(.numericText())
            }
            .frame(width: PopupMetrics.donutSize, height: PopupMetrics.donutSize)
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: model.fractions)
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: model.costMicros)
            .onAppear {
                Motion.perform(Motion.sweep, reduced: reducedMotion) { reveal = 1 }
            }
            .accessibilityElement(children: .combine)
        }
    }
#endif
