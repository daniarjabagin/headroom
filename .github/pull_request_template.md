## What and why

<!-- One or two sentences. Link the issue it closes: Closes #123 -->

## Checklist

- [ ] The checks for every part I touched pass (see [CONTRIBUTING.md](https://github.com/daniarjabagin/headroom/blob/main/CONTRIBUTING.md#build-run-and-test)): `cargo fmt`, `cargo clippy`, `cargo test`, `make lint test` in `shell/gnome` or `shell/plasma`, `swift test` in `shell/macos`
- [ ] New behaviour comes with tests; a bug fix starts with a failing test
- [ ] UI changes include screenshots in light and dark themes
- [ ] User-visible changes have a line under `[Unreleased]` in `CHANGELOG.md`
- [ ] No tokens, API keys, emails or unredacted logs in code, fixtures or screenshots
