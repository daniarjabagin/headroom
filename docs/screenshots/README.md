# Screenshots

The images in this folder are used by the top-level [README](../../README.md). Every Linux shot is
rendered from mock data (`shell/gnome/dev/mock-daemon.py --scenario showcase`), never from a real
account.

| file | what | how it was made |
| --- | --- | --- |
| `popup-dark.png` | GNOME popup, dark | headless GNOME Shell 50 at scale 2 with the mock daemon |
| `popup-light.png` | GNOME popup, light | same, with `color-scheme` set to `prefer-light` |
| `gnome-settings.png` | GNOME preferences window, General tab | same session, `gnome-extensions prefs` |
| `plasma.png` | Plasma widget popup, dark | `make -C shell/plasma preview STATE=…` with the showcase state, cropped to the popup |
| `macos-dark.png` | macOS menu-bar popup, dark | **still to take on a Mac**, see below |
| `macos-light.png` | macOS menu-bar popup, light | **still to take on a Mac**, see below |
| `macos-settings.png` | macOS preferences window | **still to take on a Mac**, see below (optional, not linked yet) |

## macOS shots to take

These cannot be rendered on Linux, so the owner takes them on a Mac with the installed app.

1. **Use accounts without personal data.** Nothing in the popup may show a personal email or a
   company name. Rename accounts in Settings → Accounts (for example "work" and "personal"), or
   use accounts whose labels are already neutral. Check the Keychain prompts are gone before the
   shot.
2. **Show a full popup.** Two to four providers, a spend donut with several slices (open the
   popup on a day with some usage), and ideally one limit with a warning pace.
3. **`macos-dark.png`**: System Settings → Appearance → Dark. Click the Headroom item in the menu
   bar, press `⌘⇧4`, then `Space`, and click the popup. This captures the window with its shadow on a
   transparent background.
4. **`macos-light.png`**: the same in Appearance → Light.
5. **`macos-settings.png`** (optional): open Settings… (`⌘,`) on the General tab and take the same
   `⌘⇧4`, `Space` window shot.
6. Screenshots land on the Desktop as `Screenshot … .png`. Rename them to the names above, copy
   them into `docs/screenshots/`, and keep them small: Retina shots are fine as they are; if a file
   is over about 1 MB, run it through `oxipng -o 4` or ImageOptim.

The README shows `macos-dark.png` and `macos-light.png` in the screenshot row through a `<picture>`
element, so both files are needed; until they exist that cell shows a broken image.
