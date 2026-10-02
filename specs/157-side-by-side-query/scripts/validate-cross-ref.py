#!/usr/bin/env python3
"""Validate SPEC-157 pack: relative md links, LAW/F/EC id coverage."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ERRORS: list[str] = []
WARNINGS: list[str] = []

LINK_RE = re.compile(r"\[([^\]]+)\]\(([^)]+)\)")
LAW_RE = re.compile(r"LAW-157-(\d+)")
FINDING_RE = re.compile(r"F-157-(\d+)")
EC_RE = re.compile(r"EC-157-(\d+)")
US_RE = re.compile(r"US-157-(\d+)")


def resolve_link(src: Path, target: str) -> Path | None:
    if target.startswith(("http://", "https://", "mailto:", "#")):
        return None
    path_part = target.split("#", 1)[0]
    if not path_part:
        return None
    # Markdown often percent-encodes (), [] in paths for link parsers.
    from urllib.parse import unquote

    return (src.parent / unquote(path_part)).resolve()


def check_links() -> None:
    for md in ROOT.rglob("*.md"):
        text = md.read_text(encoding="utf-8")
        for _label, target in LINK_RE.findall(text):
            dest = resolve_link(md, target)
            if dest is None:
                continue
            if dest.exists():
                continue
            try:
                dest.relative_to(ROOT)
                in_pack = True
            except ValueError:
                in_pack = False
            if in_pack:
                ERRORS.append(
                    f"Broken in-pack link in {md.relative_to(ROOT)}: ({target})"
                )
            elif target.startswith("../"):
                WARNINGS.append(
                    f"External/missing link in {md.relative_to(ROOT)}: ({target})"
                )


def check_coverage() -> None:
    laws_doc = (ROOT / "01-first-principles.md").read_text(encoding="utf-8")
    laws = {int(n) for n in LAW_RE.findall(laws_doc)}
    expected_laws = set(range(1, 15))
    if expected_laws - laws:
        ERRORS.append(f"Missing LAW-157 in 01: {sorted(expected_laws - laws)}")

    xref = (ROOT / "11-cross-ref.md").read_text(encoding="utf-8")
    xref_laws = {int(n) for n in LAW_RE.findall(xref)}
    if expected_laws - xref_laws:
        ERRORS.append(
            f"11-cross-ref missing laws: {sorted(expected_laws - xref_laws)}"
        )

    surfaces = (ROOT / "02-surfaces.md").read_text(encoding="utf-8")
    findings = {int(n) for n in FINDING_RE.findall(surfaces)}
    expected_findings = set(range(1, 10))
    if expected_findings - findings:
        ERRORS.append(
            f"Missing F-157 in 02-surfaces: {sorted(expected_findings - findings)}"
        )
    xref_findings = {int(n) for n in FINDING_RE.findall(xref)}
    if expected_findings - xref_findings:
        ERRORS.append(
            f"11-cross-ref missing findings: {sorted(expected_findings - xref_findings)}"
        )

    ec_doc = (ROOT / "08-edge-cases.md").read_text(encoding="utf-8")
    ecs = {int(n) for n in EC_RE.findall(ec_doc)}
    expected_ecs = set(range(1, 33))
    if expected_ecs - ecs:
        ERRORS.append(f"Missing EC-157 in 08: {sorted(expected_ecs - ecs)}")

    matrix = (ROOT / "10-e2e-test-matrix.md").read_text(encoding="utf-8")
    # Matrix references ECs as bare numbers in the EC(s) column — require each
    # EC appear either as EC-157-NN or as a zero-padded token in a gate row.
    missing_in_matrix: list[int] = []
    for n in sorted(ecs):
        padded = f"{n:02d}"
        if (
            f"EC-157-{padded}" in matrix
            or f"EC-157-{n}" in matrix
            or re.search(rf"\|\s*{padded}\s*\|", matrix)
            or re.search(rf"(?:^|[^\d]){padded}(?:,|\s|\|)", matrix)
        ):
            continue
        # Also accept unpadded in comma lists like "13,14"
        if re.search(rf"(?:^|[^\d]){n}(?:,|\s|\|)", matrix):
            continue
        missing_in_matrix.append(n)
    if missing_in_matrix:
        ERRORS.append(
            f"ECs in 08 missing from 10 matrix coverage: {missing_in_matrix}"
        )

    product = (ROOT / "03-product-spec.md").read_text(encoding="utf-8")
    stories = {int(n) for n in US_RE.findall(product)}
    expected_stories = set(range(1, 11))
    if expected_stories - stories:
        ERRORS.append(
            f"Missing US-157 in 03: {sorted(expected_stories - stories)}"
        )

    required_files = [
        "README.md",
        "00-why.md",
        "01-first-principles.md",
        "02-surfaces.md",
        "03-product-spec.md",
        "04-ux-ui-spec.md",
        "05-frontend-architecture.md",
        "06-data-and-db-contract.md",
        "07-ai-engineer-spec.md",
        "08-edge-cases.md",
        "09-implementation-plan.md",
        "10-e2e-test-matrix.md",
        "11-cross-ref.md",
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
    print(f"OK: SPEC-157 pack valid ({len(WARNINGS)} warning(s)). Root={ROOT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
