public enum ToastKind: Sendable, Hashable {
    case success, failure
}

public struct PopupToast: Sendable, Hashable, Identifiable {
    public static let visibleSeconds = 2

    public let id: Int
    public let kind: ToastKind
    public let title: String
    public let detail: String?

    public static func imageShared(id: Int, saved: Bool, strings: UIStrings) -> PopupToast {
        PopupToast(
            id: id, kind: .success, title: strings.text(PopupExtraText.imageCopied),
            detail: saved ? strings.text(PopupExtraText.savedToPictures) : nil)
    }

    public static func textCopied(id: Int, strings: UIStrings) -> PopupToast {
        PopupToast(id: id, kind: .success, title: strings.text(PopupExtraText.textCopied), detail: nil)
    }

    public static func shareFailed(id: Int, strings: UIStrings) -> PopupToast {
        PopupToast(id: id, kind: .failure, title: strings.text(PopupExtraText.shareFailed), detail: nil)
    }
}
