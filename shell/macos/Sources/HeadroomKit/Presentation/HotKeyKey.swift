public struct HotKeyKey: Sendable, Hashable {
    public let code: UInt32

    public var isFunctionKey: Bool { Self.functionCodes.contains(code) }

    static func named(_ name: Substring) -> HotKeyKey? {
        guard let code = codes[name.lowercased()] else { return nil }
        return HotKeyKey(code: code)
    }

    private static let letters: [String: UInt32] = [
        "a": 0x00, "s": 0x01, "d": 0x02, "f": 0x03, "h": 0x04, "g": 0x05, "z": 0x06, "x": 0x07, "c": 0x08,
        "v": 0x09, "b": 0x0B, "q": 0x0C, "w": 0x0D, "e": 0x0E, "r": 0x0F, "y": 0x10, "t": 0x11, "o": 0x1F,
        "u": 0x20, "i": 0x22, "p": 0x23, "l": 0x25, "j": 0x26, "k": 0x28, "n": 0x2D, "m": 0x2E,
    ]

    private static let digits: [String: UInt32] = [
        "1": 0x12, "2": 0x13, "3": 0x14, "4": 0x15, "6": 0x16, "5": 0x17, "9": 0x19, "7": 0x1A, "8": 0x1C,
        "0": 0x1D,
    ]

    private static let punctuation: [String: UInt32] = [
        "equal": 0x18, "minus": 0x1B, "bracketright": 0x1E, "bracketleft": 0x21, "apostrophe": 0x27,
        "semicolon": 0x29, "backslash": 0x2A, "comma": 0x2B, "slash": 0x2C, "period": 0x2F, "grave": 0x32,
    ]

    private static let editing: [String: UInt32] = [
        "return": 0x24, "tab": 0x30, "space": 0x31, "backspace": 0x33, "escape": 0x35, "home": 0x73,
        "page_up": 0x74, "delete": 0x75, "end": 0x77, "page_down": 0x79, "left": 0x7B, "right": 0x7C,
        "down": 0x7D, "up": 0x7E,
    ]

    private static let functions: [String: UInt32] = [
        "f1": 0x7A, "f2": 0x78, "f3": 0x63, "f4": 0x76, "f5": 0x60, "f6": 0x61, "f7": 0x62, "f8": 0x64,
        "f9": 0x65, "f10": 0x6D, "f11": 0x67, "f12": 0x6F, "f13": 0x69, "f14": 0x6B, "f15": 0x71, "f16": 0x6A,
        "f17": 0x40, "f18": 0x4F, "f19": 0x50, "f20": 0x5A,
    ]

    private static let functionCodes = Set(functions.values)

    private static let codes = [letters, digits, punctuation, editing, functions].reduce(into: [:]) { table, part in
        table.merge(part) { first, _ in first }
    }
}
