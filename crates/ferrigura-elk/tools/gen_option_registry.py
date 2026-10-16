#!/usr/bin/env python3
"""Generate `src/core/data/registry.rs` from the ELK melk metadata files.

ELK declares its layout options in `*.melk` files; the metacompiler turns them
into Java classes at build time, and those classes are not in the repository
at the pinned commit. elkjs registers the options of the algorithms it
bundles, and option suffixes are only unique over that whole registry, so the
generator reads the same set of files that elkjs builds from.

Usage: gen_option_registry.py <path-to-elk-checkout> src/core/data/registry.rs
"""

import os
import re
import sys

# The melk files elkjs 0.9.3 compiles (its build copies these plugins).
MELK_FILES = [
    "plugins/org.eclipse.elk.core/src/org/eclipse/elk/core/Core.melk",
    "plugins/org.eclipse.elk.core/src/org/eclipse/elk/core/labels/Labels.melk",
    "plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/Layered.melk",
    "plugins/org.eclipse.elk.alg.force/src/org/eclipse/elk/alg/force/Force.melk",
    "plugins/org.eclipse.elk.alg.force/src/org/eclipse/elk/alg/force/stress/Stress.melk",
    "plugins/org.eclipse.elk.alg.mrtree/src/org/eclipse/elk/alg/mrtree/Tree.melk",
    "plugins/org.eclipse.elk.alg.radial/src/org/eclipse/elk/alg/radial/Radial.melk",
    "plugins/org.eclipse.elk.alg.rectpacking/src/org/eclipse/elk/alg/rectpacking/RectPacking.melk",
    "plugins/org.eclipse.elk.alg.spore/src/org/eclipse/elk/alg/spore/SPOrE.melk",
    "plugins/org.eclipse.elk.alg.disco/src/org/eclipse/elk/alg/disco/Disco.melk",
    "plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/compaction/Polyomino.melk",
]

OPTION_RE = re.compile(
    r"^\s*(?:advanced\s+|hidden\s+)*option\s+(\^?\w+)\s*:\s*([\w.<>\[\], ]+?)\s*\{"
)
# `supports <option> = <value>` in an algorithm block: the algorithm's own default for an option.
SUPPORTS_RE = re.compile(r"(?m)^\s*supports\s+([\w.^]+)\s*=\s*(.+?)\s*$")
# The algorithm whose `supports` defaults the crate applies (the only one it runs).
LAYERED_MELK = "plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/Layered.melk"
GROUP_RE = re.compile(r"^\s*group\s+(\w+)\s*\{")
PREFIX_RE = re.compile(r"^\s*idPrefix\s+([\w.]+)")
DEFAULT_RE = re.compile(r"^\s*default\s*=\s*(.+?)\s*$")


def strip_comments(text):
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return re.sub(r"//[^\n]*", "", text)


def strip_comments_java(text):
    """Remove Java block and line comments and string literals' contents are kept as-is."""
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return re.sub(r"(?<!:)//[^\n]*", "", text)


def OPTIONS_SOURCE_ENUMS(elk_root):
    """Enum names referenced by option kinds in the registry the generator builds."""
    kinds = java_types(elk_root)
    names = set()
    for relative in MELK_FILES:
        for _, type_text, _, _, _ in parse_melk(os.path.join(elk_root, relative)):
            kind, arg = classify(type_text, kinds)
            if kind in ("Enum", "EnumSet"):
                names.add(arg)
    return sorted(names)


def parse_melk(path):
    """Yield (id, type_text, default_text, legacy_ids, targets) for each option.

    Blocks are brace-matched.

    The melk grammar nests groups and options in braces; a line-based reader loses
    track of the nesting when a block closes on an unexpected line, so the reader
    keeps a stack of open blocks. String literals are blanked first, so braces in
    descriptions do not count.
    """
    with open(path, encoding="utf-8") as handle:
        text = strip_comments(handle.read())
    text = re.sub(r'"(?:\\.|[^"\\])*"', '""', text)
    prefix_match = re.search(r"idPrefix\s+([\w.^]+)", text)
    prefix = prefix_match.group(1).replace("^", "") if prefix_match else None
    stack = []  # entries: dict(kind, name, type, start, groups)
    statement_start = 0
    results = []
    group_names = []
    for index, char in enumerate(text):
        if char == "{":
            header = text[statement_start:index].strip()
            statement_start = index + 1
            group_match = re.search(r"(?:^|\s)group\s+(\w+)\s*$", header)
            option_match = re.search(
                r"(?:^|\s)option\s+(\^?\w+)\s*:\s*([\w.<>\[\], ]+?)\s*$", header
            )
            if group_match:
                stack.append({"kind": "group", "name": group_match.group(1), "start": index})
                group_names.append(group_match.group(1))
            elif option_match and "programmatic" not in header:
                # Programmatic options are Java-only and are not registered with elkjs.
                stack.append(
                    {
                        "kind": "option",
                        "name": option_match.group(1).replace("^", ""),
                        "type": option_match.group(2).strip(),
                        "start": index,
                        "groups": list(group_names),
                    }
                )
            else:
                stack.append({"kind": "other", "start": index})
        elif char == "}":
            statement_start = index + 1
            if not stack:
                continue
            block = stack.pop()
            if block["kind"] == "group":
                group_names.pop()
            elif block["kind"] == "option":
                body = text[block["start"] + 1 : index]
                default_match = re.search(r"(?m)^\s*default\s*=\s*(.+?)\s*$", body)
                default = default_match.group(1) if default_match else None
                legacy_match = re.search(r"(?m)^\s*legacyIds\s+(.+?)\s*$", body)
                legacy = (
                    [item for item in re.split(r"[\s,]+", legacy_match.group(1)) if item]
                    if legacy_match
                    else []
                )
                targets_match = re.search(r"(?m)^\s*targets\s+(.+?)\s*$", body)
                targets = (
                    [item for item in re.split(r"[\s,]+", targets_match.group(1)) if item]
                    if targets_match
                    else []
                )
                option_id = ".".join([prefix] + block["groups"] + [block["name"]])
                results.append((option_id, block["type"], default, legacy, targets))
        elif char == ";":
            statement_start = index + 1
    if prefix is None:
        raise SystemExit(f"no idPrefix in {path}")
    return results


def java_types(elk_root):
    """Map Java simple names to 'enum' for every enum declared in the sources.

    Enums are often nested (`BoxLayoutProvider.PackingMode`), so the name comes
    from the declaration, not from the file name.
    """
    kinds = {}
    for dirpath, _, files in os.walk(os.path.join(elk_root, "plugins")):
        if "/src/" not in dirpath + "/":
            continue
        for name in files:
            if not name.endswith(".java"):
                continue
            with open(os.path.join(dirpath, name), encoding="utf-8", errors="replace") as handle:
                body = handle.read()
            for match in re.finditer(r"\benum\s+(\w+)", body):
                kinds[match.group(1)] = "enum"
    return kinds


PRIMITIVES = {
    "boolean": "Boolean",
    "int": "Int",
    "Integer": "Int",
    "double": "Double",
    "float": "Double",
    "String": "String",
}


def classify(type_text, kinds):
    if type_text in PRIMITIVES:
        return ("Primitive", PRIMITIVES[type_text])
    match = re.fullmatch(r"EnumSet<(\w+)>", type_text)
    if match:
        return ("EnumSet", match.group(1))
    match = re.fullmatch(r"List<(\w+)>", type_text)
    if match:
        return ("List", match.group(1))
    if kinds.get(type_text) == "enum":
        return ("Enum", type_text)
    return ("Object", type_text)


def rust_str(text):
    return '"' + text.replace("\\", "\\\\").replace('"', '\\"') + '"'


def layered_parent_defaults(elk_root, targets_by_id):
    """The `supports <option> = <value>` lines of Layered.melk for options that target parents.

    elkjs 0.9.3 reads these values for unset options that target parents: an unset edgeRouting
    behaves as ORTHOGONAL. Options that target nodes keep their own default: an unset
    portAlignment.default behaves as DISTRIBUTED, not the layered JUSTIFIED.
    """
    with open(os.path.join(elk_root, LAYERED_MELK), encoding="utf-8") as handle:
        text = strip_comments(handle.read())
    defaults = []
    for name, value in SUPPORTS_RE.findall(text):
        option_id = name.replace("^", "")
        if option_id not in targets_by_id:
            raise SystemExit(f"Layered.melk supports unknown option {option_id}")
        if "parents" in targets_by_id[option_id]:
            defaults.append((option_id, value))
    return defaults


def main():
    elk_root, out_path = sys.argv[1], sys.argv[2]
    kinds = java_types(elk_root)
    rows = []
    seen = set()
    targets_by_id = {}
    for relative in MELK_FILES:
        path = os.path.join(elk_root, relative)
        for option_id, type_text, default_text, legacy_ids, targets in parse_melk(path):
            if option_id == "org.eclipse.elk.algorithm":
                # Declared as `^algorithm` in Core.melk; the Algorithm row below replaces it.
                continue
            if option_id in seen:
                raise SystemExit(f"duplicate option id {option_id}")
            seen.add(option_id)
            targets_by_id[option_id] = targets
            kind, arg = classify(type_text, kinds)
            rows.append((option_id, kind, arg, type_text, default_text, os.path.basename(relative), legacy_ids))
    layered_defaults = layered_parent_defaults(elk_root, targets_by_id)
    # `org.eclipse.elk.algorithm` is not declared in any melk file: the metacompiler
    # generates it from the `algorithm` blocks. Its value is a layout algorithm id.
    rows.append(("org.eclipse.elk.algorithm", "Algorithm", "LayoutAlgorithmData", "LayoutAlgorithmData", None, "generated", []))
    rows.sort(key=lambda row: row[0])
    lines = [
        "// @generated by crates/ferrigura-elk/tools/gen_option_registry.py from the ELK melk",
        "// files at commit 9bc93474e (see docs-port/elk/SOURCES.md). Do not edit by hand.",
        "",
        "use super::{OptionKind, OptionSpec};",
        "",
        "/// Every option elkjs 0.9.3 registers, sorted by id.",
        "pub static OPTIONS: &[OptionSpec] = &[",
    ]
    for option_id, kind, arg, type_text, default_text, source, legacy_ids in rows:
        if kind == "Primitive":
            kind_text = f"OptionKind::{arg}"
        elif kind == "EnumSet":
            kind_text = f"OptionKind::EnumSet({rust_str(arg)})"
        elif kind == "List":
            kind_text = f"OptionKind::List({rust_str(arg)})"
        elif kind == "Enum":
            kind_text = f"OptionKind::Enum({rust_str(arg)})"
        elif kind == "Algorithm":
            kind_text = "OptionKind::Algorithm"
        else:
            kind_text = f"OptionKind::Object({rust_str(arg)})"
        default_rust = rust_str(default_text) if default_text is not None else "None"
        if default_text is not None:
            default_rust = f"Some({default_rust})"
        lines.append(
            f"    OptionSpec {{ id: {rust_str(option_id)}, kind: {kind_text}, "
            f"melk_type: {rust_str(type_text)}, melk_default: {default_rust}, source: {rust_str(source)}, "
            f"legacy_ids: &[{', '.join(rust_str(item) for item in legacy_ids)}] }},"
        )
    lines.append("];")
    lines.append("")
    lines.append("/// Layered.melk `supports <option> = <value>` for the options that target parents: the value an")
    lines.append("/// unset option reads as, in place of the melk default. See `default_value`.")
    lines.append("pub static LAYERED_PARENT_DEFAULTS: &[(&str, &str)] = &[")
    for option_id, value in layered_defaults:
        lines.append(f"    ({rust_str(option_id)}, {rust_str(value)}),")
    lines.append("];")
    lines.append("")
    with open(out_path, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines))
    print(f"wrote {len(rows)} options and {len(layered_defaults)} layered defaults to {out_path}")


if __name__ == "__main__":
    main()
