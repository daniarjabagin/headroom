public struct HotKeyModifiers: OptionSet, Sendable, Hashable {
    public let rawValue: UInt32

    public init(rawValue: UInt32) {
        self.rawValue = rawValue
    }

    public static let command = HotKeyModifiers(rawValue: 1 << 8)
    public static let shift = HotKeyModifiers(rawValue: 1 << 9)
    public static let option = HotKeyModifiers(rawValue: 1 << 11)
    public static let control = HotKeyModifiers(rawValue: 1 << 12)

    static func named(_ name: Substring) -> HotKeyModifiers? {
        switch name.lowercased() {
        case "super", "meta", "primary", "command", "cmd": .command
        case "control", "ctrl", "ctl": .control
        case "alt", "mod1", "option": .option
        case "shift": .shift
        default: nil
        }
    }
}

public struct HotKeyChord: Sendable, Hashable {
    public let modifiers: HotKeyModifiers
    public let key: HotKeyKey

    public var carbonModifiers: UInt32 { modifiers.rawValue }
    public var carbonKeyCode: UInt32 { key.code }

    public static func parse(_ accelerator: String) -> HotKeyChord? {
        var modifiers: HotKeyModifiers = []
        var rest = Substring(accelerator)
        while rest.first == "<" {
            guard let close = rest.firstIndex(of: ">"),
                let modifier = HotKeyModifiers.named(rest[rest.index(after: rest.startIndex)..<close])
            else { return nil }
            modifiers.insert(modifier)
            rest = rest[rest.index(after: close)...]
        }
        guard let key = HotKeyKey.named(rest), !modifiers.isEmpty || key.isFunctionKey else { return nil }
        return HotKeyChord(modifiers: modifiers, key: key)
    }
}
