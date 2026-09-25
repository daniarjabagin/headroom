import QtQuick
import QtQuick.Layouts
import QtTest
import "../package/contents/ui"

TestCase {
    id: suite

    width: 800
    height: 800
    when: windowShown

    readonly property var links: [
        {
            icon: "network-connect",
            label: "Status page",
            menuHost: "status.openai.com",
            url: "https://status.openai.com"
        },
        {
            icon: "link",
            label: "Open dashboard",
            menuHost: "chatgpt.com",
            url: "https://chatgpt.com"
        }
    ]

    function entries(menu) {
        const found = [];
        for (let index = 0; index < menu.count; ++index)
            found.push(menu.itemAt(index));
        return found;
    }

    function openedMenu(starred) {
        const menu = createTemporaryObject(accountMenu, suite, {
            providerName: "Codex",
            links,
            starred,
            canStar: true,
            canShare: true,
            lang: "en"
        }) as AccountMenu;
        menu.popup(suite, 0, 0);
        tryVerify(() => menu.opened);
        return menu;
    }

    function test_account_menu_fits_its_widest_entry() {
        const menu = openedMenu(false);
        const widest = Math.max(...entries(menu).map(item => item.implicitWidth));
        verify(menu.width >= widest + menu.leftPadding + menu.rightPadding - 1);
        verify(entries(menu).every(item => item.width >= item.implicitWidth - 1));
    }

    function test_account_menu_stays_inside_the_window() {
        const menu = createTemporaryObject(accountMenu, suite, {
            providerName: "Codex",
            links,
            starred: false,
            canStar: true,
            canShare: true,
            lang: "en"
        }) as AccountMenu;
        menu.popup(suite, suite.width - 10, suite.height - 10);
        tryVerify(() => menu.opened);
        verify(menu.y >= 0);
        verify(menu.y + menu.height <= suite.height);
        verify(menu.x + menu.width <= suite.width);
    }

    function test_account_menu_shows_link_hosts_as_detail() {
        const menu = openedMenu(false);
        const linkEntries = entries(menu).filter(item => item.detail !== undefined && item.detail !== "");
        compare(linkEntries.map(item => [item.text, item.detail]), [["Status page", "status.openai.com"], ["Open dashboard", "chatgpt.com"]]);
    }

    function test_always_show_uses_the_star_like_the_tray() {
        const menu = openedMenu(false);
        const star = entries(menu).find(item => item.objectName === "menuStar");
        compare(star.text, "Always show");
        compare(star.leadIcon, "non-starred-symbolic");
        verify(!star.checkable);
        menu.starred = true;
        compare(star.leadIcon, "starred-symbolic");
    }

    function test_unit_menu_stacks_subtitles_beside_the_check() {
        const title = createTemporaryObject(unitTitle, suite) as UnitTitle;
        title.clicked();
        const menu = Array.from(title.data).find(child => child.objectName === "unitMenu");
        tryVerify(() => menu.opened);
        const rows = entries(menu);
        waitForItemPolished(rows[1].contentItem);
        compare(rows.map(row => row.marked), [false, true, false]);
        compare(rows[1].detail, "Input, output and cache tokens");
        const lead = rows[1].contentItem.children[0];
        const texts = rows[1].contentItem.children[1];
        verify(lead.visible);
        compare(lead.opacity, 1);
        compare(rows[0].contentItem.children[0].opacity, 0);
        verify(texts.x >= lead.x + lead.width);
        verify(texts.children[1].visible);
    }

    function test_segments_never_shrink_below_their_labels() {
        const row = createTemporaryObject(narrowRow, suite) as RowLayout;
        const control = row.children[0] as SegmentedControl;
        verify(control.widest > 0);
        verify(control.width >= control.naturalWidth);
        verify(control.segmentWidth >= control.widest);
    }

    Component {
        id: accountMenu

        AccountMenu {}
    }

    Component {
        id: unitTitle

        UnitTitle {
            unit: "tokens"
            lang: "en"
        }
    }

    Component {
        id: narrowRow

        RowLayout {
            width: 40

            SegmentedControl {
                Layout.preferredWidth: 20
                options: [
                    {
                        value: "auto",
                        label: "Автоматически"
                    },
                    {
                        value: "24h",
                        label: "24-часовой"
                    }
                ]
                current: "auto"
            }
        }
    }
}
