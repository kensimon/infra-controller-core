#!/usr/bin/env python3
"""Inline `args.rs` and `cmd.rs` modules into sibling `<module>.rs` files.

This targets module directories under crates/admin-cli/src that contain:
- mod.rs
- args.rs
- cmd.rs or cmds.rs

For each such directory `<parent>/<name>/`, this script creates:
- `<parent>/<name>.rs`

The resulting file is based on `mod.rs`, replacing `mod args;` / `pub mod args;`
and `mod cmd;` / `pub mod cmd;` with inline module blocks containing the contents
of args.rs/cmd.rs (with their leading license header removed).

Import handling:
- Collect all top-level `use` / `pub use` lines from mod.rs, args.rs, cmd.rs.
- Keep mod.rs imports at file scope.
- Hoist imports shared by args.rs and cmd.rs to file scope.
- Insert `use super::*;` at the start of each inlined submodule.
- Keep only submodule imports that are not hoisted.

By default, the script also removes `mod.rs`, `args.rs`, and `cmd.rs`, and deletes
now-empty module directories.
"""

from __future__ import annotations

import argparse
import pathlib
import re
import sys
from dataclasses import dataclass


LICENSE_BLOCK_RE = re.compile(
    r"\A\s*/\*(?:(?!\*/).)*SPDX-License-Identifier:(?:(?!\*/).)*\*/\s*",
    re.DOTALL,
)
USE_LINE_RE = re.compile(r"^(pub\s+)?use\s+.*;\s*$")


@dataclass
class ModuleTriplet:
    mod_rs: pathlib.Path
    args_rs: pathlib.Path
    cmd_rs: pathlib.Path
    cmd_mod_name: str

    @property
    def module_dir(self) -> pathlib.Path:
        return self.mod_rs.parent

    @property
    def out_rs(self) -> pathlib.Path:
        return self.module_dir.with_suffix(".rs")


def strip_leading_license(src: str) -> str:
    m = LICENSE_BLOCK_RE.match(src)
    if not m:
        return src.lstrip("\n")
    return src[m.end() :].lstrip("\n")


def split_license(src: str) -> tuple[str, str]:
    m = LICENSE_BLOCK_RE.match(src)
    if not m:
        return "", src
    return src[: m.end()].rstrip(), src[m.end() :]


def indent_block(src: str, spaces: int = 4) -> str:
    pad = " " * spaces
    lines = src.rstrip().splitlines()
    if not lines:
        return ""
    return "\n".join(f"{pad}{line}" if line else "" for line in lines)


def replace_module_decl(mod_src: str, module_name: str, module_body: str) -> str:
    # Match one declaration line: `mod foo;` or `pub mod foo;`.
    pat = re.compile(
        rf"(?m)^(?P<indent>[ \t]*)(?P<vis>pub\s+)?mod\s+{re.escape(module_name)}\s*;\s*$"
    )
    m = pat.search(mod_src)
    if not m:
        raise ValueError(
            f"could not find module declaration for `{module_name}` in mod.rs"
        )

    indent = m.group("indent")
    vis = m.group("vis") or ""
    body = indent_block(module_body, spaces=4)

    replacement = (
        f"{indent}{vis}mod {module_name} {{\n"
        f"{body}\n"
        f"{indent}}}"
    )
    return mod_src[: m.start()] + replacement + mod_src[m.end() :]


def extract_top_level_uses(src: str) -> tuple[list[str], str]:
    uses: list[str] = []
    body_lines: list[str] = []

    for line in src.splitlines(keepends=True):
        stripped = line.strip()
        if USE_LINE_RE.match(stripped) and line[: len(line) - len(line.lstrip())] == "":
            uses.append(stripped)
            continue
        body_lines.append(line)

    return dedupe_preserve(uses), "".join(body_lines)


def dedupe_preserve(lines: list[str]) -> list[str]:
    seen: set[str] = set()
    out: list[str] = []
    for line in lines:
        if line in seen:
            continue
        seen.add(line)
        out.append(line)
    return out


def inline_module_body(inner_src: str, local_uses: list[str]) -> str:
    body = inner_src.strip("\n")
    out: list[str] = ["use super::*;"]

    if local_uses:
        out.extend(local_uses)

    if body:
        out.append("")
        out.append(body)

    return "\n".join(out)


def normalize_blank_lines(src: str) -> str:
    return re.sub(r"\n{3,}", "\n\n", src)


def find_triplets(root: pathlib.Path) -> list[ModuleTriplet]:
    triplets: list[ModuleTriplet] = []
    for mod_rs in sorted(root.rglob("mod.rs")):
        module_dir = mod_rs.parent
        args_rs = module_dir / "args.rs"
        cmd_rs = module_dir / "cmd.rs"
        cmds_rs = module_dir / "cmds.rs"
        if not args_rs.exists():
            continue

        if cmd_rs.exists():
            triplets.append(
                ModuleTriplet(
                    mod_rs=mod_rs,
                    args_rs=args_rs,
                    cmd_rs=cmd_rs,
                    cmd_mod_name="cmd",
                )
            )
            continue

        if cmds_rs.exists():
            triplets.append(
                ModuleTriplet(
                    mod_rs=mod_rs,
                    args_rs=args_rs,
                    cmd_rs=cmds_rs,
                    cmd_mod_name="cmds",
                )
            )
    return triplets


def convert_one(t: ModuleTriplet, *, dry_run: bool) -> None:
    mod_src = t.mod_rs.read_text(encoding="utf-8")
    args_src = t.args_rs.read_text(encoding="utf-8")
    cmd_src = t.cmd_rs.read_text(encoding="utf-8")

    license_block, mod_rest = split_license(mod_src)
    args_rest = strip_leading_license(args_src)
    cmd_rest = strip_leading_license(cmd_src)

    mod_uses, mod_body = extract_top_level_uses(mod_rest)
    args_uses, args_body = extract_top_level_uses(args_rest)
    cmd_uses, cmd_body = extract_top_level_uses(cmd_rest)

    mod_use_set = set(mod_uses)
    cmd_use_set = set(cmd_uses)
    shared_child_uses = [line for line in args_uses if line in cmd_use_set and line not in mod_use_set]

    hoisted_uses = dedupe_preserve(mod_uses + shared_child_uses)
    hoisted_set = set(hoisted_uses)

    args_local_uses = [line for line in args_uses if line not in hoisted_set]
    cmd_local_uses = [line for line in cmd_uses if line not in hoisted_set]

    args_inline = inline_module_body(args_body, args_local_uses)
    cmd_inline = inline_module_body(cmd_body, cmd_local_uses)

    updated = replace_module_decl(mod_body, "args", args_inline)
    updated = replace_module_decl(updated, t.cmd_mod_name, cmd_inline)
    updated = normalize_blank_lines(updated).lstrip("\n")

    chunks: list[str] = []
    if license_block:
        chunks.append(license_block)
    if hoisted_uses:
        chunks.append("\n".join(hoisted_uses))
    chunks.append(updated)
    updated = "\n\n".join(chunk for chunk in chunks if chunk.strip())

    if not updated.endswith("\n"):
        updated += "\n"

    if dry_run:
        print(f"[dry-run] would write {t.out_rs}")
        print(f"[dry-run] would remove {t.mod_rs}")
        print(f"[dry-run] would remove {t.args_rs}")
        print(f"[dry-run] would remove {t.cmd_rs}")
        return

    t.out_rs.write_text(updated, encoding="utf-8")
    t.mod_rs.unlink()
    t.args_rs.unlink()
    t.cmd_rs.unlink()

    # Remove the directory if it is now empty.
    try:
        t.module_dir.rmdir()
    except OSError:
        pass

    print(f"converted {t.module_dir} -> {t.out_rs}")


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--root",
        type=pathlib.Path,
        default=pathlib.Path("crates/admin-cli/src"),
        help="Root path to scan for module directories (default: crates/admin-cli/src)",
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="Print planned changes without writing files",
    )
    return p.parse_args()


def main() -> int:
    args = parse_args()
    root: pathlib.Path = args.root

    if not root.exists():
        print(f"error: root path does not exist: {root}", file=sys.stderr)
        return 1

    triplets = find_triplets(root)
    if not triplets:
        print("no matching module directories found")
        return 0

    for triplet in triplets:
        try:
            convert_one(triplet, dry_run=args.dry_run)
        except Exception as exc:  # noqa: BLE001
            print(f"error converting {triplet.module_dir}: {exc}", file=sys.stderr)
            return 1

    print(f"done: processed {len(triplets)} module director{'y' if len(triplets) == 1 else 'ies'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
