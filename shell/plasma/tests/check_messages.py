#!/usr/bin/env python3
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCES = ROOT / "package" / "contents"
LOGIC = SOURCES / "ui" / "logic"
AGGREGATE = LOGIC / "Russian.js"
CATALOGS = sorted(LOGIC.glob("Russian?*.js"))
LITERAL = r'"((?:[^"\\]|\\.)*)"'
SINGLE = re.compile(r"\b(?:tr|N)\(\s*(?:[\w.]+\s*,\s*)?" + LITERAL)
PLURAL = re.compile(r"\btrn\(\s*[\w.]+\s*,\s*" + LITERAL + r"\s*,\s*" + LITERAL)
PLACEHOLDER = re.compile(r"\{(\w+)\}")
UNMARKED_THROW = re.compile(r"\bnew\s+\w*Error\((?!\s*(?:I18n\.)?N\()")


def unescape(literal):
    return json.loads(f'"{literal}"')


def entries(path):
    text = path.read_text(encoding="utf-8")
    start = text.index("{")
    end = text.rindex("}") + 1
    return json.loads(text[start:end])


def catalog():
    merged, duplicates = {}, []
    for path in CATALOGS:
        for msgid, entry in entries(path).items():
            if msgid in merged:
                duplicates.append(f"{path.name}: duplicate entry {msgid!r}")
            merged[msgid] = entry
    return merged, duplicates


def sources():
    skipped = set(CATALOGS) | {AGGREGATE}
    for path in sorted(SOURCES.rglob("*")):
        if path.suffix in {".qml", ".js"} and path not in skipped:
            yield path


def unmarked_throws():
    found = []
    for path in sources():
        text = path.read_text(encoding="utf-8")
        for match in UNMARKED_THROW.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            found.append(f"{path.name}:{line}: thrown error message must be a msgid marked with N()")
    return found


def used_messages():
    singles, plurals = {}, {}
    for path in sources():
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
    messages, found = catalog()
    singles, plurals = used_messages()
    found.extend(unmarked_throws())
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
    found.extend(f"Russian catalogs: unused entry {msgid!r}" for msgid in sorted(unused))
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
