import QtQuick
import QtTest
import org.kde.kirigami as Kirigami
import "../package/contents/ui"
import "../package/contents/ui/logic/Account.js" as Account
import "../package/contents/ui/logic/Tokens.js" as Tokens

TestCase {
    id: suite

    function account(notices) {
        return {
            id: "grok:1",
            provider: "grok",
            providerName: "Grok",
            status: "ok",
            error: null,
            notices
        };
    }

    function test_tones_map_to_notice_kinds() {
        const rows = Account.notices("en", account([
            {
                tone: "critical",
                text: "Out of credits"
            },
            {
                tone: "warning",
                text: "Shared limit"
            },
            {
                tone: "neutral",
                text: "Extra usage on, cap $25.00"
            },
            {
                tone: "good",
                text: "All set"
            }
        ]), false, []);
        compare(rows.map(row => row.kind), ["error", "warning", "info", "info"]);
    }

    function test_info_notices_leave_the_plates() {
        const rows = Account.notices("en", account([
            {
                tone: "neutral",
                text: "Extra usage on, cap $25.00"
            },
            {
                tone: "warning",
                text: "Shared limit"
            }
        ]), false, []);
        compare(Account.plates(rows).map(row => row.title), ["Shared limit"]);
        compare(Account.infoLines(rows).map(row => row.title), ["Extra usage on, cap $25.00"]);
    }

    function test_info_line_is_plain_secondary_caption() {
        const line = createTemporaryObject(lineComponent, suite);
        compare(line.role, "caption");
        compare(line.emphasis, "secondary");
        compare(String(line.color), String(Tokens.secondaryText(Kirigami.Theme)));
        compare(line.wrapMode, Text.Wrap);
    }

    width: 200
    height: 200
    visible: true
    when: windowShown

    Component {
        id: lineComponent

        NoticeLine {
            text: "Extra usage on, cap $25.00"
        }
    }
}
