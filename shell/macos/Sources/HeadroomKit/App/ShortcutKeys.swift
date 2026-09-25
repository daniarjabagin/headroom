enum ShortcutKeys {
    static let escape: UInt16 = 0x35
    static let erasers: Set<UInt16> = [0x33, 0x75]

    static let special: [UInt16: String] = [
        0x24: "Return", 0x30: "Tab", 0x31: "space", 0x33: "BackSpace", 0x35: "Escape", 0x75: "Delete",
        0x73: "Home", 0x77: "End", 0x74: "Page_Up", 0x79: "Page_Down",
        0x7B: "Left", 0x7C: "Right", 0x7D: "Down", 0x7E: "Up",
        0x7A: "F1", 0x78: "F2", 0x63: "F3", 0x76: "F4", 0x60: "F5", 0x61: "F6",
        0x62: "F7", 0x64: "F8", 0x65: "F9", 0x6D: "F10", 0x67: "F11", 0x6F: "F12",
        0x69: "F13", 0x6B: "F14", 0x71: "F15", 0x6A: "F16", 0x40: "F17", 0x4F: "F18", 0x50: "F19", 0x5A: "F20",
    ]

    static let layoutKeys: [UInt16: String] = [
        0x00: "a", 0x01: "s", 0x02: "d", 0x03: "f", 0x04: "h", 0x05: "g", 0x06: "z", 0x07: "x",
        0x08: "c", 0x09: "v", 0x0B: "b", 0x0C: "q", 0x0D: "w", 0x0E: "e", 0x0F: "r", 0x10: "y",
        0x11: "t", 0x1F: "o", 0x20: "u", 0x22: "i", 0x23: "p", 0x25: "l", 0x26: "j", 0x28: "k",
        0x2D: "n", 0x2E: "m",
        0x12: "1", 0x13: "2", 0x14: "3", 0x15: "4", 0x17: "5", 0x16: "6", 0x1A: "7", 0x1C: "8",
        0x19: "9", 0x1D: "0",
        0x18: "equal", 0x1B: "minus", 0x1E: "bracketright", 0x21: "bracketleft", 0x27: "apostrophe",
        0x29: "semicolon", 0x2A: "backslash", 0x2B: "comma", 0x2C: "slash", 0x2F: "period", 0x32: "grave",
    ]

    static let punctuation: [String: String] = [
        "equal": "=", "minus": "-", "bracketright": "]", "bracketleft": "[", "apostrophe": "'",
        "semicolon": ";", "backslash": "\\", "comma": ",", "slash": "/", "period": ".", "grave": "`",
    ]

    static let symbols: [String: String] = [
        "Return": "↩", "Tab": "⇥", "space": "Space", "BackSpace": "⌫", "Escape": "⎋", "Delete": "⌦",
        "Home": "↖", "End": "↘", "Page_Up": "⇞", "Page_Down": "⇟",
        "Left": "←", "Right": "→", "Down": "↓", "Up": "↑",
    ]

    static func isFunctionKey(_ key: String) -> Bool {
        guard key.count > 1, key.first == "F", let number = Int(key.dropFirst()) else { return false }
        return (1...20).contains(number)
    }
}
