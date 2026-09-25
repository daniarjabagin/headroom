import Foundation

public enum HeaderMenuKind: Sendable, Hashable {
    case refresh, hide, star, link(ProviderLinkKind), share, copyText
}

public struct HeaderMenuItem: Sendable, Hashable, Identifiable {
    public let kind: HeaderMenuKind
    public let title: String
    public let detail: String?
    public let url: URL?
    public let checked: Bool

    public var id: String { "\(kind)" }
}

public struct HeaderLinkButton: Sendable, Hashable, Identifiable {
    public let kind: ProviderLinkKind
    public let url: URL
    public let tip: String

    public var id: ProviderLinkKind { kind }
}

public struct HeaderMenuModel: Sendable, Hashable {
    static let buttonOrder: [ProviderLinkKind] = [.status, .usage, .dashboard]
    static let menuOrder: [ProviderLinkKind] = [.status, .dashboard, .usage]

    public let groups: [[HeaderMenuItem]]
    public let buttons: [HeaderLinkButton]

    public static func make(
        _ section: AccountSectionModel, providerName: String, links: ProviderLinks?, display: DisplaySettings,
        features: DaemonFeatures, strings: UIStrings
    ) -> HeaderMenuModel {
        let urls = distinctLinks(links)
        let manage = [
            item(.refresh, strings.fill(PopupExtraText.refreshProvider, ["provider": providerName])),
            item(.hide, strings.text(PopupExtraText.hideFromPopup)),
            features.release06
                ? item(.star, strings.text(PopupExtraText.alwaysShow), checked: isStarred(section, display)) : nil,
        ]
        let linkItems = menuOrder.compactMap { kind in
            urls[kind].map { item(.link(kind), title(kind, strings), detail: $0.host, url: $0) }
        }
        let share =
            shares(section)
            ? [
                item(.share, strings.text(PopupExtraText.shareImage)),
                item(.copyText, strings.text(PopupExtraText.copyText)),
            ] : []
        let buttons = buttonOrder.compactMap { kind in
            urls[kind].map { url in
                HeaderLinkButton(
                    kind: kind, url: url,
                    tip: strings.fill(PopupExtraText.linkTip, ["title": title(kind, strings), "host": url.host ?? ""]))
            }
        }
        return HeaderMenuModel(
            groups: [manage.compactMap { $0 }, linkItems, share].filter { !$0.isEmpty }, buttons: buttons)
    }

    public static func isStarred(_ section: AccountSectionModel, _ display: DisplaySettings) -> Bool {
        !section.memberIDs.isEmpty && section.memberIDs.allSatisfy(display.starredAccounts.contains)
    }

    public static func starredAfterToggle(_ section: AccountSectionModel, _ display: DisplaySettings) -> [String] {
        let others = display.starredAccounts.filter { !section.memberIDs.contains($0) }
        return isStarred(section, display) ? others : others + section.memberIDs
    }

    static func shares(_ section: AccountSectionModel) -> Bool {
        switch section.body {
        case .blocked: false
        case .limits(let limits): !limits.windows.isEmpty
        case .combined(let limits): !limits.windows.isEmpty
        }
    }

    static func distinctLinks(_ links: ProviderLinks?) -> [ProviderLinkKind: URL] {
        guard let links else { return [:] }
        var urls: [ProviderLinkKind: URL] = [:]
        for kind in ProviderLinkKind.allCases {
            guard let url = links.url(kind), !urls.values.contains(url) else { continue }
            urls[kind] = url
        }
        return urls
    }

    static func title(_ kind: ProviderLinkKind, _ strings: UIStrings) -> String {
        switch kind {
        case .status: strings.text(PopupExtraText.statusPage)
        case .dashboard: strings.text(PopupExtraText.openDashboard)
        case .usage: strings.text(PopupExtraText.usagePage)
        }
    }

    private static func item(
        _ kind: HeaderMenuKind, _ title: String, detail: String? = nil, url: URL? = nil, checked: Bool = false
    ) -> HeaderMenuItem {
        HeaderMenuItem(kind: kind, title: title, detail: detail, url: url, checked: checked)
    }
}
