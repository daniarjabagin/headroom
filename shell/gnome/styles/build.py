#!/usr/bin/env python3
import json
import re
import sys
from pathlib import Path

STYLES = Path(__file__).resolve().parent
EXTENSION = STYLES.parent
TOKEN = re.compile(r"var\(--([a-z0-9-]+)\)")
OUTPUTS = {
    "stylesheet-light.css": "light",
    "stylesheet-dark.css": "dark",
    "stylesheet.css": "dark",
}


def render(template, tokens, variant):
    def substitute(match):
        name = match.group(1)
        if name not in tokens:
            sys.exit(f"styles/build.py: token '{name}' is missing from the '{variant}' theme")
        return tokens[name]

    return TOKEN.sub(substitute, template)


def check_same_keys(themes):
    names = {variant: set(tokens) for variant, tokens in themes.items()}
    reference = names["light"]
    for variant, keys in names.items():
        if keys != reference:
            sys.exit(f"styles/build.py: '{variant}' tokens differ: {sorted(keys ^ reference)}")


def main():
    themes = json.loads((STYLES / "tokens.json").read_text())
    check_same_keys(themes)
    template = (STYLES / "stylesheet.template.css").read_text()
    for filename, variant in OUTPUTS.items():
        (EXTENSION / filename).write_text(render(template, themes[variant], variant))


if __name__ == "__main__":
    main()
