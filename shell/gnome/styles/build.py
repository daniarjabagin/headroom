#!/usr/bin/env python3
import json
import re
import sys
from pathlib import Path

STYLES = Path(__file__).resolve().parent
EXTENSION = STYLES.parent
TOKEN = re.compile(r"var\(--([a-z0-9-]+)\)")
RULE = re.compile(r"([^{}]+)\{([^{}]*)\}")
THEME_CLASS = "headroom-theme-{variant}"
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


def declarations(block):
    pairs = [line.strip().rstrip(";").split(":", 1) for line in block.split(";") if ":" in line]
    return [(name.strip(), value.strip()) for name, value in pairs]


def themed_declarations(block):
    pairs = declarations(block)
    themed = {name for name, value in pairs if TOKEN.search(value)}
    return [(name, value) for name, value in pairs if name in themed]


def scoped_selectors(selectors, variant):
    scope = "." + THEME_CLASS.format(variant=variant)
    scoped = []
    for selector in (part.strip() for part in selectors.split(",")):
        scoped.append(f"{scope} {selector}")
        scoped.append(f"{scope}{selector}")
    return ",\n".join(scoped)


def theme_overrides(template, variant):
    rules = []
    for selectors, block in RULE.findall(template):
        pairs = themed_declarations(block)
        if not pairs:
            continue
        body = "".join(f"    {name}: {value};\n" for name, value in pairs)
        rules.append(f"{scoped_selectors(selectors, variant)} {{\n{body}}}\n")
    return "\n".join(rules)


def stylesheet(template, themes, variant):
    other = "light" if variant == "dark" else "dark"
    base = render(template, themes[variant], variant)
    overrides = render(theme_overrides(template, other), themes[other], other)
    return f"{base}\n{overrides}"


def main():
    themes = json.loads((STYLES / "tokens.json").read_text())
    check_same_keys(themes)
    template = "\n".join(part.read_text() for part in sorted((STYLES / "template").glob("*.css")))
    for filename, variant in OUTPUTS.items():
        (EXTENSION / filename).write_text(stylesheet(template, themes, variant))


if __name__ == "__main__":
    main()
