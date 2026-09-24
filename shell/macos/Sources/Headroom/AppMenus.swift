#if canImport(AppKit)
    import AppKit
    import HeadroomKit

    @MainActor
    final class AppMenus: NSObject {
        var onRefresh: (@MainActor () -> Void)?
        var onSettings: (@MainActor () -> Void)?

        func statusMenu(_ strings: UIStrings) -> NSMenu {
            let menu = NSMenu()
            for item in commandItems(strings) { menu.addItem(item) }
            return menu
        }

        func mainMenu(_ strings: UIStrings) -> NSMenu {
            let main = NSMenu()
            main.addItem(submenu("Headroom", commandItems(strings)))
            main.addItem(submenu(strings.text(MenuText.edit), editItems(strings)))
            main.addItem(submenu(strings.text(MenuText.window), windowItems(strings)))
            return main
        }

        private func commandItems(_ strings: UIStrings) -> [NSMenuItem] {
            [
                targeted(strings.text(SettingsText.refreshNow), #selector(refreshChosen), key: "r"),
                targeted(strings.text(SettingsText.settingsMenu), #selector(settingsChosen), key: ","),
                .separator(),
                NSMenuItem(
                    title: strings.text(.quit), action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q"),
            ]
        }

        private func editItems(_ strings: UIStrings) -> [NSMenuItem] {
            let redo = NSMenuItem(title: strings.text(MenuText.redo), action: Selector(("redo:")), keyEquivalent: "z")
            redo.keyEquivalentModifierMask = [.command, .shift]
            return [
                NSMenuItem(title: strings.text(MenuText.undo), action: Selector(("undo:")), keyEquivalent: "z"), redo,
                .separator(),
                NSMenuItem(title: strings.text(MenuText.cut), action: #selector(NSText.cut(_:)), keyEquivalent: "x"),
                NSMenuItem(title: strings.text(MenuText.copy), action: #selector(NSText.copy(_:)), keyEquivalent: "c"),
                NSMenuItem(
                    title: strings.text(MenuText.paste), action: #selector(NSText.paste(_:)), keyEquivalent: "v"),
                NSMenuItem(
                    title: strings.text(MenuText.selectAll), action: #selector(NSText.selectAll(_:)), keyEquivalent: "a"
                ),
            ]
        }

        private func windowItems(_ strings: UIStrings) -> [NSMenuItem] {
            [
                NSMenuItem(
                    title: strings.text(MenuText.minimize), action: #selector(NSWindow.performMiniaturize(_:)),
                    keyEquivalent: "m"),
                NSMenuItem(
                    title: strings.text(MenuText.closeWindow), action: #selector(NSWindow.performClose(_:)),
                    keyEquivalent: "w"),
            ]
        }

        private func submenu(_ title: String, _ items: [NSMenuItem]) -> NSMenuItem {
            let menu = NSMenu(title: title)
            for item in items { menu.addItem(item) }
            let holder = NSMenuItem(title: title, action: nil, keyEquivalent: "")
            holder.submenu = menu
            return holder
        }

        private func targeted(_ title: String, _ action: Selector, key: String) -> NSMenuItem {
            let item = NSMenuItem(title: title, action: action, keyEquivalent: key)
            item.target = self
            return item
        }

        @objc private func refreshChosen() {
            onRefresh?()
        }

        @objc private func settingsChosen() {
            onSettings?()
        }
    }
#endif
