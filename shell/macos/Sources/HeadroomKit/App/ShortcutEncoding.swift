public struct ShortcutModifiers: OptionSet, Sendable, Hashable {
    public let rawValue: Int

    public init(rawValue: Int) {
        self.rawValue = rawValue
    }

    public static let control = ShortcutModifiers(rawValue: 1 << 0)
    public static let option = ShortcutModifiers(rawValue: 1 << 1)
    public static let shift = ShortcutModifiers(rawValue: 1 << 2)
    public static let command = ShortcutModifiers(rawValue: 1 << 3)

    static let ordered: [(modifier: ShortcutModifiers, accelerator: String, symbol: String)] = [
        (.control, "<Control>", "⌃"), (.option, "<Alt>", "⌥"), (.shift, "<Shift>", "⇧"), (.command, "<Super>", "⌘"),
    ]
}

public enum ShortcutCapture: Sendable, Hashable {
    case accelerator(String)
    case cancel
    case clear
    case needsModifier
    case unsupported
}

public enum ShortcutEncoding {
    public static func capture(modifiers: ShortcutModifiers, keyCode: UInt16, characters: String?) -> ShortcutCapture {
        if modifiers.isEmpty, keyCode == ShortcutKeys.escape { return .cancel }
        if modifiers.isEmpty, ShortcutKeys.erasers.contains(keyCode) { return .clear }
        guard let key = keyName(keyCode: keyCode, characters: characters) else { return .unsupported }
        guard ShortcutKeys.isFunctionKey(key) || !modifiers.subtracting(.shift).isEmpty else { return .needsModifier }
        let encoded = accelerator(modifiers: modifiers, key: key)
        return HotKeyChord.parse(encoded) == nil ? .unsupported : .accelerator(encoded)
    }

    public static func accelerator(modifiers: ShortcutModifiers, key: String) -> String {
        ShortcutModifiers.ordered.filter { modifiers.contains($0.modifier) }.map(\.accelerator).joined() + key
    }

    public static func keyName(keyCode: UInt16, characters: String?) -> String? {
        if let special = ShortcutKeys.special[keyCode] { return special }
        if let letter = asciiLetter(characters) { return letter }
        return ShortcutKeys.layoutKeys[keyCode]
    }

    public static func symbols(_ accelerator: String) -> String? {
        guard let parsed = parse(accelerator) else { return nil }
        let prefix = ShortcutModifiers.ordered.filter { parsed.modifiers.contains($0.modifier) }.map(\.symbol)
        return prefix.joined() + keySymbol(parsed.key)
    }

    static func parse(_ accelerator: String) -> (modifiers: ShortcutModifiers, key: String)? {
        var rest = Substring(accelerator)
        var modifiers = ShortcutModifiers()
        while rest.first == "<" {
            guard let close = rest.firstIndex(of: ">"),
                let modifier = modifier(named: rest[rest.index(after: rest.startIndex)..<close])
            else { return nil }
            modifiers.insert(modifier)
            rest = rest[rest.index(after: close)...]
        }
        guard !rest.isEmpty, rest.allSatisfy(isKeyNameCharacter) else { return nil }
        return (modifiers, String(rest))
    }

    private static func modifier(named name: Substring) -> ShortcutModifiers? {
        guard let named = HotKeyModifiers.named(name) else { return nil }
        let pairs: [(HotKeyModifiers, ShortcutModifiers)] = [
            (.control, .control), (.option, .option), (.shift, .shift), (.command, .command),
        ]
        return pairs.first { named.contains($0.0) }?.1
    }

    private static func keySymbol(_ key: String) -> String {
        if let symbol = ShortcutKeys.symbols[key] { return symbol }
        if let character = ShortcutKeys.punctuation[key] { return character }
        return key.count == 1 ? key.uppercased() : key
    }

    private static func asciiLetter(_ characters: String?) -> String? {
        guard let characters, characters.count == 1, let scalar = characters.unicodeScalars.first,
            scalar.isASCII, scalar.properties.isAlphabetic
        else { return nil }
        return characters.lowercased()
    }

    private static func isKeyNameCharacter(_ character: Character) -> Bool {
        character.isASCII && (character.isLetter || character.isNumber || character == "_")
    }
}
