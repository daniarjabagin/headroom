extension DisplayFormatter {
    private static let halfCentMicros: Int64 = 5000
    private static let ellipsis: Character = "…"

    public func costPerMTok(micros: Int64?) -> String {
        guard let micros else { return Self.dash }
        guard micros <= 0 || micros >= Self.halfCentMicros else {
            return "<\(exactUSD(micros: 2 * Self.halfCentMicros))"
        }
        return exactUSD(micros: micros)
    }

    public func projectName(_ project: String?, maxCharacters: Int) -> String {
        guard let project else { return strings.text(SpendOptionText.noProject) }
        return Self.middleEllipsis(project, maxCharacters: maxCharacters)
    }

    public static func middleEllipsis(_ path: String, maxCharacters: Int) -> String {
        let characters = Array(path)
        guard characters.count > maxCharacters else { return path }
        guard maxCharacters > 1 else { return maxCharacters == 1 ? String(ellipsis) : "" }
        let tail = lastComponent(characters)
        if !tail.isEmpty, tail.count + 1 < maxCharacters {
            return String(characters.prefix(maxCharacters - 1 - tail.count)) + String(ellipsis) + String(tail)
        }
        let kept = maxCharacters - 1
        return String(characters.prefix((kept + 1) / 2)) + String(ellipsis) + String(characters.suffix(kept / 2))
    }

    private static func lastComponent(_ characters: [Character]) -> ArraySlice<Character> {
        guard let slash = characters.dropLast().lastIndex(of: "/"), slash > 0 else { return [] }
        return characters[slash...]
    }
}
