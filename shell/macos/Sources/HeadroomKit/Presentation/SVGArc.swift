import Foundation

struct SVGArc {
    let from: PlanePoint
    let to: PlanePoint
    let radiusX: Double
    let radiusY: Double
    let rotation: Double
    let largeArc: Bool
    let sweep: Bool

    private struct Ellipse {
        let center: PlanePoint
        let radiusX: Double
        let radiusY: Double
        let cosine: Double
        let sine: Double

        func point(_ angle: Double) -> PlanePoint {
            let x = radiusX * cos(angle)
            let y = radiusY * sin(angle)
            return PlanePoint(x: cosine * x - sine * y + center.x, y: sine * x + cosine * y + center.y)
        }

        func derivative(_ angle: Double) -> PlanePoint {
            let x = -radiusX * sin(angle)
            let y = radiusY * cos(angle)
            return PlanePoint(x: cosine * x - sine * y, y: sine * x + cosine * y)
        }
    }

    func cubics() -> [SVGPathCommand] {
        guard from != to else { return [] }
        guard radiusX != 0, radiusY != 0 else { return [.line(to)] }
        let phi = rotation * .pi / 180
        let cosine = cos(phi)
        let sine = sin(phi)
        let halfX = (from.x - to.x) / 2
        let halfY = (from.y - to.y) / 2
        let primeX = cosine * halfX + sine * halfY
        let primeY = -sine * halfX + cosine * halfY
        var rx = abs(radiusX)
        var ry = abs(radiusY)
        let lambda = (primeX * primeX) / (rx * rx) + (primeY * primeY) / (ry * ry)
        if lambda > 1 {
            rx *= lambda.squareRoot()
            ry *= lambda.squareRoot()
        }
        let numerator = rx * rx * ry * ry - rx * rx * primeY * primeY - ry * ry * primeX * primeX
        let denominator = rx * rx * primeY * primeY + ry * ry * primeX * primeX
        let coefficient = (max(0, numerator / denominator)).squareRoot() * (largeArc == sweep ? -1 : 1)
        let centerPrime = PlanePoint(x: coefficient * rx * primeY / ry, y: -coefficient * ry * primeX / rx)
        let center = PlanePoint(
            x: cosine * centerPrime.x - sine * centerPrime.y + (from.x + to.x) / 2,
            y: sine * centerPrime.x + cosine * centerPrime.y + (from.y + to.y) / 2)
        let startVector = PlanePoint(x: (primeX - centerPrime.x) / rx, y: (primeY - centerPrime.y) / ry)
        let endVector = PlanePoint(x: (-primeX - centerPrime.x) / rx, y: (-primeY - centerPrime.y) / ry)
        let ellipse = Ellipse(center: center, radiusX: rx, radiusY: ry, cosine: cosine, sine: sine)
        return segments(
            ellipse, start: Self.angle(PlanePoint(x: 1, y: 0), startVector), sweepAngle: sweepAngle(startVector, endVector))
    }

    private func sweepAngle(_ start: PlanePoint, _ end: PlanePoint) -> Double {
        var delta = Self.angle(start, end)
        if !sweep && delta > 0 { delta -= 2 * .pi }
        if sweep && delta < 0 { delta += 2 * .pi }
        return delta
    }

    private func segments(_ ellipse: Ellipse, start: Double, sweepAngle: Double) -> [SVGPathCommand] {
        let count = max(1, Int((abs(sweepAngle) / (.pi / 2)).rounded(.up)))
        let step = sweepAngle / Double(count)
        let handle = 4.0 / 3 * tan(step / 4)
        return (0..<count).map { index in
            let first = start + Double(index) * step
            let second = first + step
            let begin = ellipse.point(first)
            let end = index == count - 1 ? to : ellipse.point(second)
            return .cubic(
                begin + handle * ellipse.derivative(first), end - handle * ellipse.derivative(second), end)
        }
    }

    private static func angle(_ lhs: PlanePoint, _ rhs: PlanePoint) -> Double {
        atan2(lhs.x * rhs.y - lhs.y * rhs.x, lhs.x * rhs.x + lhs.y * rhs.y)
    }
}
