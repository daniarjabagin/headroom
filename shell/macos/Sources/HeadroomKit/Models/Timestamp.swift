import Foundation

public struct Timestamp: Sendable, Hashable, Comparable, Decodable {
    public let date: Date

    public init(date: Date) {
        self.date = date
    }

    public init?(rfc3339 text: String) {
        guard let date = RFC3339.parse(text) else { return nil }
        self.date = date
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.singleValueContainer()
        let text = try container.decode(String.self)
        guard let date = RFC3339.parse(text) else {
            throw DecodingError.dataCorruptedError(
                in: container, debugDescription: "invalid RFC 3339 timestamp \(text)")
        }
        self.date = date
    }

    public static func < (lhs: Timestamp, rhs: Timestamp) -> Bool {
        lhs.date < rhs.date
    }

    public func seconds(since other: Timestamp) -> TimeInterval {
        date.timeIntervalSince(other.date)
    }
}
