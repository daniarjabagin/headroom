# Provider logos

Copies of `assets/providers/*.svg` (Simple Icons, CC0-1.0), renamed to the provider id that uses
them. See `assets/providers/NOTICE.md` for sources and trademark notes. The app draws them as
monochrome glyphs in the text color, except `claude.svg`, which uses the brand color `#D97757`.

| provider id | source file |
| --- | --- |
| `codex` | `openai.svg` |
| `claude` | `claude.svg` |
| `opencode` | `opencode.svg` |
| `openrouter` | `openrouter.svg` |
| `zai` | `zdotai.svg` |
| `kimi` | `kimi.svg` |
| `minimax` | `minimax.svg` |
| `cline` | `cline.svg` |
| `copilot` | `githubcopilot.svg` |
| `cursor` | `cursor.svg` |
| `antigravity` | `google.svg` |
| `ollama` | `ollama.svg` |
| `warp` | `warp.svg` |
| `poe` | `poe.svg` |
| `deepseek` | `deepseek.svg` |
| `moonshot` | `moonshotai.svg` |

`grok`, `devin` and `kilo` have no logo; the app draws a monogram for them.

## Series colors of the newer providers

`ProviderStyle` gives every provider a spend-ring color (light / dark). The five providers added
after the design-system table use their brand hue where it is free, otherwise the nearest hue no
other provider uses:

| provider id | light | dark | reason |
| --- | --- | --- | --- |
| `kilo` | `#B59A00` | `#F8F675` | Kilo Code brand yellow, darkened for light backgrounds |
| `warp` | `#005A9E` | `#6CCBFF` | Warp brand azure, set apart from Kimi's sky blue by lightness |
| `poe` | `#B42BC9` | `#DE7BF0` | Poe's purple pushed to magenta, clear of OpenRouter and Copilot |
| `deepseek` | `#2C3FC2` | `#A3B1FF` | DeepSeek brand blue, deeper/lighter than Antigravity and OpenRouter |
| `moonshot` | `#475A78` | `#A5B4CC` | Moonshot AI's black mark as a blue slate, apart from the neutral grays |
