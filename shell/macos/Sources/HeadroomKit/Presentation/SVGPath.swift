import Foundation

public enum SVGPathCommand: Sendable, Hashable {
    case move(PlanePoint)
    case line(PlanePoint)
    case cubic(PlanePoint, PlanePoint, PlanePoint)
    case close
}

public enum SVGPath {
    public static func parse(_ data: String) throws(SVGPathError) -> [SVGPathCommand] {
        var scanner = SVGScanner(data)
        var builder = SVGPathBuilder()
        var repeating: Character?
        while !scanner.isAtEnd {
            guard let command = scanner.command() ?? repeating else { throw .missingMoveTo }
            guard builder.started || command == "M" || command == "m" else { throw .missingMoveTo }
            try builder.apply(command, &scanner)
            repeating = command == "M" ? "L" : command == "m" ? "l" : (command == "Z" || command == "z") ? nil : command
            if repeating != nil && !scanner.hasNumber() { repeating = nil }
        }
        return builder.commands
    }
}

struct SVGPathBuilder {
    private(set) var commands: [SVGPathCommand] = []
    private var current = PlanePoint(x: 0, y: 0)
    private var subpathStart = PlanePoint(x: 0, y: 0)
    private var lastCubicControl: PlanePoint?
    private var lastQuadControl: PlanePoint?

    var started: Bool { !commands.isEmpty }

    mutating func apply(_ command: Character, _ scanner: inout SVGScanner) throws(SVGPathError) {
        let relative = command.isLowercase
        let origin = relative ? current : PlanePoint(x: 0, y: 0)
        switch command.lowercased() {
        case "m": moveTo(origin + (try point(&scanner)))
        case "l": lineTo(origin + (try point(&scanner)))
        case "h": lineTo(PlanePoint(x: (relative ? current.x : 0) + (try scanner.number()), y: current.y))
        case "v": lineTo(PlanePoint(x: current.x, y: (relative ? current.y : 0) + (try scanner.number())))
        case "c": try cubic(&scanner, origin: origin, smooth: false)
        case "s": try cubic(&scanner, origin: origin, smooth: true)
        case "q": try quadratic(&scanner, origin: origin, smooth: false)
        case "t": try quadratic(&scanner, origin: origin, smooth: true)
        case "a": try arc(&scanner, origin: origin)
        case "z": close()
        default: throw .unknownCommand(command)
        }
    }

    private func point(_ scanner: inout SVGScanner) throws(SVGPathError) -> PlanePoint {
        let x = try scanner.number()
        return PlanePoint(x: x, y: try scanner.number())
    }

    private mutating func moveTo(_ point: PlanePoint) {
        commands.append(.move(point))
        current = point
        subpathStart = point
        clearControls()
    }

    private mutating func lineTo(_ point: PlanePoint) {
        commands.append(.line(point))
        current = point
        clearControls()
    }

    private mutating func close() {
        commands.append(.close)
        current = subpathStart
        clearControls()
    }

    private mutating func clearControls() {
        lastCubicControl = nil
        lastQuadControl = nil
    }

    private mutating func cubic(_ scanner: inout SVGScanner, origin: PlanePoint, smooth: Bool) throws(SVGPathError) {
        let first = smooth ? reflected(lastCubicControl) : origin + (try point(&scanner))
        let second = origin + (try point(&scanner))
        let end = origin + (try point(&scanner))
        commands.append(.cubic(first, second, end))
        current = end
        lastCubicControl = second
        lastQuadControl = nil
    }

    private mutating func quadratic(_ scanner: inout SVGScanner, origin: PlanePoint, smooth: Bool) throws(SVGPathError) {
        let control = smooth ? reflected(lastQuadControl) : origin + (try point(&scanner))
        let end = origin + (try point(&scanner))
        let twoThirds = 2.0 / 3
        commands.append(.cubic(current + twoThirds * (control - current), end + twoThirds * (control - end), end))
        current = end
        lastQuadControl = control
        lastCubicControl = nil
    }

    private mutating func arc(_ scanner: inout SVGScanner, origin: PlanePoint) throws(SVGPathError) {
        let radiusX = try scanner.number()
        let radiusY = try scanner.number()
        let rotation = try scanner.number()
        let largeArc = try scanner.flag()
        let sweep = try scanner.flag()
        let end = origin + (try point(&scanner))
        let arc = SVGArc(
            from: current, to: end, radiusX: radiusX, radiusY: radiusY, rotation: rotation, largeArc: largeArc,
            sweep: sweep)
        commands.append(contentsOf: arc.cubics())
        current = end
        clearControls()
    }

    private func reflected(_ control: PlanePoint?) -> PlanePoint {
        guard let control else { return current }
        return current + (current - control)
    }
}
