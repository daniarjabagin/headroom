import Foundation

public struct SVGIcon: Sendable, Hashable {
    public let origin: PlanePoint
    public let width: Double
    public let height: Double
    public let commands: [SVGPathCommand]

    public static func parse(_ document: String) throws(SVGPathError) -> SVGIcon {
        let paths = attributes(named: "d", in: document)
        guard !paths.isEmpty else { throw .missingPathData }
        var commands: [SVGPathCommand] = []
        for path in paths { commands += try SVGPath.parse(path) }
        let box = attributes(named: "viewBox", in: document).first.map(viewBox) ?? nil
        return SVGIcon(
            origin: PlanePoint(x: box?[0] ?? 0, y: box?[1] ?? 0), width: box?[2] ?? 24, height: box?[3] ?? 24,
            commands: commands)
    }

    static func attributes(named name: String, in document: String) -> [String] {
        let marker = " \(name)=\""
        var results: [String] = []
        var rest = document[...]
        while let start = rest.range(of: marker) {
            let valueStart = start.upperBound
            guard let end = rest[valueStart...].firstIndex(of: "\"") else { break }
            results.append(String(rest[valueStart..<end]))
            rest = rest[rest.index(after: end)...]
        }
        return results
    }

    private static func viewBox(_ value: String) -> [Double]? {
        let numbers = value.split(whereSeparator: { $0 == " " || $0 == "," }).compactMap { Double($0) }
        guard numbers.count == 4, numbers[2] > 0, numbers[3] > 0 else { return nil }
        return numbers
    }
}
