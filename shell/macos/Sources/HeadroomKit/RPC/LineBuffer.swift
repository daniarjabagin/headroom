import Foundation

public struct LineBuffer: Sendable {
    public static let maximumLineLength = 1 << 20

    private static let newline = UInt8(ascii: "\n")
    private static let carriageReturn = UInt8(ascii: "\r")

    private var pending = Data()

    public init() {}

    public mutating func append(_ chunk: Data) throws(DaemonError) -> [Data] {
        pending.append(chunk)
        var lines: [Data] = []
        while let end = pending.firstIndex(of: Self.newline) {
            lines.append(Self.trimmed(pending[pending.startIndex..<end]))
            pending = Data(pending[pending.index(after: end)...])
        }
        guard pending.count <= Self.maximumLineLength else { throw .lineTooLong }
        return lines.filter { !$0.isEmpty }
    }

    private static func trimmed(_ line: Data) -> Data {
        guard line.last == carriageReturn else { return Data(line) }
        return Data(line.dropLast())
    }
}
