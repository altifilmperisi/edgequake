#!/usr/bin/env python3
"""Validate SPEC-155 pack: relative md links, LAW/F/EC id coverage."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ERRORS: list[str] = []
WARNINGS: list[str] = []

LINK_RE = re.compile(r"\[([^\]]+)\]\(([^)]+)\)")
LAW_RE = re.compile(r"LAW-155-(\d+)")
FINDING_RE = re.compile(r"F-155-[A-Z]\d+")
EC_RE = re.compile(r"EC-155-(\d+)")


def resolve_link(src: Path, target: str) -> Path | None:
    if target.startswith(("http://", "https://", "mailto:", "#")):
        return None
    path_part = target.split("#", 1)[0]
    if not path_part:
        return None
    return (src.parent / path_part).resolve()


def check_links() -> None:
    md_files = list(ROOT.rglob("*.md"))
    for md in md_files:
        text = md.read_text(encoding="utf-8")
        for _label, target in LINK_RE.findall(text):
            dest = resolve_link(md, target)
            if dest is None:
                continue
            # allow links outside pack (../029-..., ../../docs)
            if not dest.exists():
                # only hard-fail for in-pack relative targets
                try:
                    dest.relative_to(ROOT)
                    in_pack = True
                except ValueError:
                    in_pack = False
                if in_pack or target.startswith("./") or (
                    not target.startswith("../") and "/" in target
                ):
                    ERRORS.append(f"Broken link in {md.relative_to(ROOT)}: ({target})")
                elif target.startswith("../"):
                    if not dest.exists():
                        WARNINGS.append(
                            f"External/missing link in {md.relative_to(ROOT)}: ({target})"
                        )


def extract_ids(path: Path, pattern: re.Pattern[str]) -> set[str]:
    text = path.read_text(encoding="utf-8")
    return set(pattern.findall(text)) if pattern.groups == 0 else set(
        m.group(0) for m in pattern.finditer(text)
    )


def check_coverage() -> None:
    laws_doc = (ROOT / "01-first-principles.md").read_text(encoding="utf-8")
    laws = {int(n) for n in LAW_RE.findall(laws_doc)}
    expected_laws = set(range(1, 15))
    missing_laws = expected_laws - laws
    if missing_laws:
        ERRORS.append(f"Missing LAW-155 in 01: {sorted(missing_laws)}")

    xref = (ROOT / "12-cross-ref.md").read_text(encoding="utf-8")
    xref_laws = {int(n) for n in LAW_RE.findall(xref)}
    if expected_laws - xref_laws:
        ERRORS.append(
            f"12-cross-ref missing laws: {sorted(expected_laws - xref_laws)}"
        )

    findings_doc = (ROOT / "04-findings.md").read_text(encoding="utf-8")
    findings = set(FINDING_RE.findall(findings_doc))
    if len(findings) < 40:
        WARNINGS.append(f"Only {len(findings)} findings found (expected ≥40)")

    ec_doc = (ROOT / "09-edge-cases.md").read_text(encoding="utf-8")
    ecs = {int(n) for n in EC_RE.findall(ec_doc)}
    matrix = (ROOT / "11-e2e-test-matrix.md").read_text(encoding="utf-8")
    # matrix uses bare numbers in first column often; also EC-155-NN
    matrix_ecs = {int(n) for n in EC_RE.findall(matrix)}
    # also lines like "| 01 |" at start of matrix rows
    bare = set()
    for line in matrix.splitlines():
        m = re.match(r"\|\s*(\d{2})\s*\|", line)
        if m:
            bare.add(int(m.group(1)))
    matrix_all = matrix_ecs | bare
    missing_in_matrix = ecs - matrix_all
    if missing_in_matrix:
        ERRORS.append(
            f"ECs in 09 missing from 11 matrix: {sorted(missing_in_matrix)}"
        )

    required_files = [
        "README.md",
        "00-why.md",
        "01-first-principles.md",
        "02-surfaces.md",
        "03-standards-crosswalk.md",
        "04-findings.md",
        "05-design-system-spec.md",
        "06-graph-studio-spec.md",
        "07-screens-spec.md",
        "08-data-contract.md",
        "09-edge-cases.md",
        "10-implementation-plan.md",
        "11-e2e-test-matrix.md",
        "12-cross-ref.md",
        "lenses/LENS-product-owner.md",
        "lenses/LENS-full-stack.md",
        "lenses/LENS-database.md",
        "lenses/LENS-ux-ui.md",
        "lenses/LENS-front.md",
        "lenses/LENS-ai-engineer.md",
    ]
    for rel in required_files:
        if not (ROOT / rel).exists():
            ERRORS.append(f"Missing required file: {rel}")


def main() -> int:
    check_links()
    check_coverage()
    for w in WARNINGS:
        print(f"WARN: {w}")
    for e in ERRORS:
        print(f"ERROR: {e}")
    if ERRORS:
        print(f"FAILED: {len(ERRORS)} error(s), {len(WARNINGS)} warning(s)")
        return 1
    print(
        f"OK: SPEC-155 pack valid ({len(WARNINGS)} warning(s)). Root={ROOT}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
