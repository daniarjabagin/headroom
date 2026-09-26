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
            let geometry = DonutGeometry(size: Double(size))
            let originX = rect.midX - size / 2
            let originY = rect.midY - size / 2
            var path = Path()
            let segments = DonutGeometry.segments(fractions.values, reveal: reveal)
            guard let segment = segments.first(where: { $0.index == index }),
                let outline = geometry.outline(segment)
            else { return path }
            for loop in outline {
                path.addLines(loop.map { CGPoint(x: originX + CGFloat($0.x), y: originY + CGFloat($0.y)) })
                path.closeSubpath()
            }
            return path
        }
    }

    struct DonutView: View {
        let model: SpendCardModel
        @State private var reveal = 0.0
        @Environment(\.headroomReducedMotion) private var reducedMotion
        @Environment(\.popupLayout) private var layout

        var body: some View {
            let size = layout.cg.donutSize
            let fit = labelFit(size: size)
            ZStack {
                ForEach(Array(model.entries.enumerated()), id: \.element.id) { index, entry in
                    DonutSliceShape(fractions: AnimatableVector(values: model.fractions), reveal: reveal, index: index)
                        .fill(entry.color.color, style: FillStyle(eoFill: true, antialiased: true))
                }
                VStack(spacing: 0) {
                    Text(model.centerAmount)
                        .font(.system(size: CGFloat(fit.fontSize), weight: .semibold, design: .rounded))
                        .lineLimit(1)
                        .minimumScaleFactor(CGFloat(RingLabelFit.minimumScale))
                        .contentTransition(.numericText())
                    if let caption = model.centerCaption {
                        Text(caption)
                            .font(.system(size: CGFloat(RingLabelFit.captionSize), weight: .medium))
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                            .minimumScaleFactor(CGFloat(RingLabelFit.minimumScale))
                    }
                }
                .monospacedDigit()
                .frame(width: CGFloat(fit.width))
            }
            .frame(width: size, height: size)
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: model.fractions)
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: model.value)
            .onAppear {
                Motion.perform(Motion.sweep, reduced: reducedMotion) { reveal = 1 }
            }
            .accessibilityElement(children: .combine)
        }

        private func labelFit(size: CGFloat) -> RingLabelFit {
            RingLabelFit.make(
                text: model.centerAmount, ringSize: Double(size), amountSize: layout.size(RingLabelFit.amountSize),
                captionSize: model.centerCaption == nil ? nil : RingLabelFit.captionSize)
        }
    }
#endif
