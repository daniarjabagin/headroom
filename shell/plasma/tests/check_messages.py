#!/usr/bin/env python3
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCES = ROOT / "package" / "contents"
CATALOG = SOURCES / "ui" / "logic" / "Russian.js"
LITERAL = r'"((?:[^"\\]|\\.)*)"'
SINGLE = re.compile(r"\b(?:tr|N)\(\s*(?:[\w.]+\s*,\s*)?" + LITERAL)
PLURAL = re.compile(r"\btrn\(\s*[\w.]+\s*,\s*" + LITERAL + r"\s*,\s*" + LITERAL)
PLACEHOLDER = re.compile(r"\{(\w+)\}")


def unescape(literal):
    return json.loads(f'"{literal}"')


def catalog():
    text = CATALOG.read_text(encoding="utf-8")
    start = text.index("{")
    end = text.rindex("}") + 1
    return json.loads(text[start:end])


def used_messages():
    singles, plurals = {}, {}
    for path in sorted(SOURCES.rglob("*")):
        if path.suffix not in {".qml", ".js"} or path == CATALOG:
            continue
        text = path.read_text(encoding="utf-8")
        for match in PLURAL.finditer(text):
            plurals[unescape(match.group(1))] = (unescape(match.group(2)), path.name)
        for match in SINGLE.finditer(text):
            singles.setdefault(unescape(match.group(1)), path.name)
    return singles, plurals


def placeholders(text):
    return set(PLACEHOLDER.findall(text))


def check_single(msgid, entry):
    if not isinstance(entry, str):
        return f"expected a string for {msgid!r}"
    if placeholders(entry) != placeholders(msgid):
        return f"placeholders differ for {msgid!r}"
    return None


def check_plural(msgid, plural, entry):
    if not isinstance(entry, list) or len(entry) != 3:
        return f"expected three plural forms for {msgid!r}"
    wanted = placeholders(msgid) | placeholders(plural)
    if any(placeholders(form) - wanted - {"count"} for form in entry):
        return f"unknown placeholder in plural forms for {msgid!r}"
    return None


def problems():
    messages = catalog()
    singles, plurals = used_messages()
    found = []
    for msgid, source in singles.items():
        if msgid not in messages:
            found.append(f"{source}: missing Russian entry for {msgid!r}")
        elif (problem := check_single(msgid, messages[msgid])) is not None:
            found.append(f"{source}: {problem}")
    for msgid, (plural, source) in plurals.items():
        if msgid not in messages:
            found.append(f"{source}: missing Russian plural entry for {msgid!r}")
        elif (problem := check_plural(msgid, plural, messages[msgid])) is not None:
            found.append(f"{source}: {problem}")
    unused = set(messages) - set(singles) - set(plurals)
    found.extend(f"Russian.js: unused entry {msgid!r}" for msgid in sorted(unused))
    return found


def main():
    found = problems()
    for problem in found:
        print(problem, file=sys.stderr)
    if found:
        return 1
    singles, plurals = used_messages()
    print(f"messages: {len(singles) + len(plurals)} msgids, all translated to Russian")
    return 0


if __name__ == "__main__":
    sys.exit(main())
