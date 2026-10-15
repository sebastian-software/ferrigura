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
    r"^\s*(?:advanced\s+|hidden\s+)*option\s+(\w+)\s*:\s*([\w.<>\[\], ]+?)\s*\{"
)
GROUP_RE = re.compile(r"^\s*group\s+(\w+)\s*\{")
PREFIX_RE = re.compile(r"^\s*idPrefix\s+([\w.]+)")
DEFAULT_RE = re.compile(r"^\s*default\s*=\s*(.+?)\s*$")


def strip_comments(text):
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return re.sub(r"//[^\n]*", "", text)


def parse_melk(path):
    """Yield (id, type_text, default_text, file_name) for each option in one melk file."""
    with open(path, encoding="utf-8") as handle:
        text = strip_comments(handle.read())
    prefix = None
    groups = []
    option = None
    for line in text.split("\n"):
        match = PREFIX_RE.match(line)
        if match and prefix is None:
            prefix = match.group(1)
            continue
        if option is None:
            match = GROUP_RE.match(line)
            if match:
                groups.append(match.group(1))
                continue
            match = OPTION_RE.match(line)
            if match:
                option = {"name": match.group(1), "type": match.group(2).strip(), "groups": list(groups), "default": None}
                continue
            if line.strip() == "}" and groups:
                groups.pop()
            continue
        match = DEFAULT_RE.match(line)
        if match and option["default"] is None:
            option["default"] = match.group(1)
        if line.strip() == "}":
            yield_id = ".".join([prefix] + option["groups"] + [option["name"]])
            yield (yield_id, option["type"], option["default"])
            option = None
    if prefix is None:
        raise SystemExit(f"no idPrefix in {path}")


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


def main():
    elk_root, out_path = sys.argv[1], sys.argv[2]
    kinds = java_types(elk_root)
    rows = []
    seen = set()
    for relative in MELK_FILES:
        path = os.path.join(elk_root, relative)
        for option_id, type_text, default_text in parse_melk(path):
            if option_id in seen:
                raise SystemExit(f"duplicate option id {option_id}")
            seen.add(option_id)
            kind, arg = classify(type_text, kinds)
            rows.append((option_id, kind, arg, type_text, default_text, os.path.basename(relative)))
    # `org.eclipse.elk.algorithm` is not declared in any melk file: the metacompiler
    # generates it from the `algorithm` blocks. Its value is a layout algorithm id.
    rows.append(("org.eclipse.elk.algorithm", "Algorithm", "LayoutAlgorithmData", "LayoutAlgorithmData", None, "generated"))
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
    for option_id, kind, arg, type_text, default_text, source in rows:
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
            f"melk_type: {rust_str(type_text)}, melk_default: {default_rust}, source: {rust_str(source)} }},"
        )
    lines.append("];")
    lines.append("")
    with open(out_path, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines))
    print(f"wrote {len(rows)} options to {out_path}")


if __name__ == "__main__":
    main()
