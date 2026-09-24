# Screenshots

The images in this folder are used by the top-level [README](../../README.md). None of them shows a
real account: the Linux shots are rendered from mock data
(`shell/gnome/dev/mock-daemon.py --scenario showcase`), the macOS shots use accounts with neutral
labels.

| file | what | how it was made |
| --- | --- | --- |
| `popup-dark.png` | GNOME popup, dark | headless GNOME Shell 50 at scale 2 with the mock daemon |
| `popup-light.png` | GNOME popup, light | same, with `color-scheme` set to `prefer-light` |
| `gnome-settings.png` | GNOME preferences window, General tab | same session, `gnome-extensions prefs` |
| `plasma.png` | Plasma widget popup, dark | `make -C shell/plasma preview STATE=…` with the showcase state, cropped to the popup |
| `mac-popup-dark.png` | macOS menu-bar popup, dark | the installed app, `⌘⇧4` then `Space` on the popup |
| `mac-popup-light.png` | macOS menu-bar popup, light | same, in System Settings → Appearance → Light |
| `mac-settings.png` | macOS preferences window, General tab (Russian UI) | same window shot of Settings… (`⌘,`) |

## Retaking a shot

- **GNOME**: `HEADLESS=1 COLOR_SCHEME=prefer-dark SCENARIO=showcase make -C shell/gnome devkit`
  runs a nested GNOME Shell with the mock daemon (drop `HEADLESS=1` for a window).
- **Plasma**: dump the showcase state and render the popup offscreen:

  ```sh
  python3 shell/gnome/dev/mock-daemon.py --dump --scenario showcase > /tmp/showcase.json
  make -C shell/plasma preview STATE=/tmp/showcase.json
  ```
- **macOS**: rename accounts in Settings → Accounts to neutral labels (for example "work" and
  "personal") so no email or company name is visible, open the popup and press `⌘⇧4`, then `Space`,
  and click the window. This captures it with its shadow on a transparent background.
- Keep the file names above and the files small: run anything over about 1 MB through
  `oxipng -o 4`.
