public enum SVGPathError: Error, Sendable, Hashable {
    case expectedNumber(offset: Int)
    case expectedFlag(offset: Int)
    case unknownCommand(Character)
    case missingMoveTo
    case missingPathData
}

struct SVGScanner {
    private let bytes: [UInt8]
    private var index = 0

    init(_ text: String) {
        bytes = Array(text.utf8)
    }

    var isAtEnd: Bool {
        var probe = self
        probe.skipSeparators()
        return probe.index >= probe.bytes.count
    }

    mutating func command() -> Character? {
        skipSeparators()
        guard index < bytes.count, Self.isLetter(bytes[index]) else { return nil }
        defer { index += 1 }
        return Character(Unicode.Scalar(bytes[index]))
    }

    mutating func hasNumber() -> Bool {
        skipSeparators()
        guard index < bytes.count else { return false }
        let byte = bytes[index]
        return Self.isDigit(byte) || byte == UInt8(ascii: "-") || byte == UInt8(ascii: "+") || byte == UInt8(ascii: ".")
    }

    mutating func number() throws(SVGPathError) -> Double {
        skipSeparators()
        let start = index
        consumeSign()
        let integerDigits = consumeDigits()
        var fractionDigits = 0
        if index < bytes.count, bytes[index] == UInt8(ascii: ".") {
            index += 1
            fractionDigits = consumeDigits()
        }
        guard integerDigits + fractionDigits > 0 else { throw .expectedNumber(offset: start) }
        consumeExponent()
        guard let value = Double(String(decoding: bytes[start..<index], as: UTF8.self)) else {
            throw .expectedNumber(offset: start)
        }
        return value
    }

    mutating func flag() throws(SVGPathError) -> Bool {
        skipSeparators()
        guard index < bytes.count else { throw .expectedFlag(offset: index) }
        let byte = bytes[index]
        guard byte == UInt8(ascii: "0") || byte == UInt8(ascii: "1") else { throw .expectedFlag(offset: index) }
        index += 1
        return byte == UInt8(ascii: "1")
    }

    private mutating func skipSeparators() {
        while index < bytes.count, Self.isSeparator(bytes[index]) { index += 1 }
    }

    private mutating func consumeSign() {
        guard index < bytes.count, bytes[index] == UInt8(ascii: "-") || bytes[index] == UInt8(ascii: "+") else { return }
        index += 1
    }

    private mutating func consumeDigits() -> Int {
        let start = index
        while index < bytes.count, Self.isDigit(bytes[index]) { index += 1 }
        return index - start
    }

    private mutating func consumeExponent() {
        guard index < bytes.count, bytes[index] == UInt8(ascii: "e") || bytes[index] == UInt8(ascii: "E") else { return }
        let mark = index
        index += 1
        consumeSign()
        if consumeDigits() == 0 { index = mark }
    }

    private static func isDigit(_ byte: UInt8) -> Bool {
        byte >= UInt8(ascii: "0") && byte <= UInt8(ascii: "9")
    }

    private static func isLetter(_ byte: UInt8) -> Bool {
        let lower = byte | 0x20
        return lower >= UInt8(ascii: "a") && lower <= UInt8(ascii: "z") && lower != UInt8(ascii: "e")
    }

    private static func isSeparator(_ byte: UInt8) -> Bool {
        byte == UInt8(ascii: " ") || byte == UInt8(ascii: ",") || byte == UInt8(ascii: "\n")
            || byte == UInt8(ascii: "\t") || byte == UInt8(ascii: "\r")
    }
}
