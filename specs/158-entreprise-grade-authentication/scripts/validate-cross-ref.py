#!/usr/bin/env python3
"""Validate SPEC-158 pack: relative md links, LAW/F/EC/US id coverage."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[1]
ERRORS: list[str] = []
WARNINGS: list[str] = []

LINK_RE = re.compile(r"\[([^\]]+)\]\(([^)]+)\)")
LAW_RE = re.compile(r"LAW-158-(\d+)")
FINDING_RE = re.compile(r"F-158-(\d+)")
EC_RE = re.compile(r"EC-158-(\d+)")
US_RE = re.compile(r"US-158-(\d+)")


def resolve_link(src: Path, target: str) -> Path | None:
    if target.startswith(("http://", "https://", "mailto:", "#")):
        return None
    path_part = target.split("#", 1)[0]
    if not path_part:
        return None
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
            elif target.startswith("../") or target.startswith("../../"):
                WARNINGS.append(
                    f"External/missing link in {md.relative_to(ROOT)}: ({target})"
                )


def check_coverage() -> None:
    laws_doc = (ROOT / "01-first-principles.md").read_text(encoding="utf-8")
    laws = {int(n) for n in LAW_RE.findall(laws_doc)}
    expected_laws = set(range(1, 15))
    if expected_laws - laws:
        ERRORS.append(f"Missing LAW-158 in 01: {sorted(expected_laws - laws)}")

    xref = (ROOT / "15-cross-ref.md").read_text(encoding="utf-8")
    xref_laws = {int(n) for n in LAW_RE.findall(xref)}
    if expected_laws - xref_laws:
        ERRORS.append(
            f"15-cross-ref missing laws: {sorted(expected_laws - xref_laws)}"
        )

    surfaces = (ROOT / "02-surfaces.md").read_text(encoding="utf-8")
    findings_doc = (ROOT / "04-findings.md").read_text(encoding="utf-8")
    findings = {int(n) for n in FINDING_RE.findall(surfaces + findings_doc)}
    expected_findings = set(range(1, 13))
    if expected_findings - findings:
        ERRORS.append(
            f"Missing F-158 in 02/04: {sorted(expected_findings - findings)}"
        )
    xref_findings = {int(n) for n in FINDING_RE.findall(xref)}
    if expected_findings - xref_findings:
        ERRORS.append(
            f"15-cross-ref missing findings: {sorted(expected_findings - xref_findings)}"
        )

    ec_doc = (ROOT / "12-edge-cases.md").read_text(encoding="utf-8")
    ecs = {int(n) for n in EC_RE.findall(ec_doc)}
    if len(ecs) < 30:
        ERRORS.append(f"Expected >= 30 distinct EC-158 in 12, found {len(ecs)}: {sorted(ecs)}")

    matrix = (ROOT / "14-e2e-test-matrix.md").read_text(encoding="utf-8")
    missing_in_matrix: list[int] = []
    for n in sorted(ecs):
        padded = f"{n:02d}"
        if (
            f"EC-158-{padded}" in matrix
            or f"EC-158-{n}" in matrix
            or re.search(rf"\|\s*{padded}\s*\|", matrix)
        ):
            continue
        missing_in_matrix.append(n)
    if missing_in_matrix:
        ERRORS.append(
            f"ECs in 12 missing from 14 matrix coverage: {missing_in_matrix}"
        )

    product = (ROOT / "03-product-spec.md").read_text(encoding="utf-8")
    stories = {int(n) for n in US_RE.findall(product)}
    expected_stories = set(range(1, 11))
    if expected_stories - stories:
        ERRORS.append(
            f"Missing US-158 in 03: {sorted(expected_stories - stories)}"
        )

    required_files = [
        "README.md",
        "00-why.md",
        "01-first-principles.md",
        "02-surfaces.md",
        "03-product-spec.md",
        "04-architecture.md",
        "04-findings.md",
        "05-keycloak-integration.md",
        "06-idp-matrix.md",
        "07-data-and-db-contract.md",
        "08-api-and-token-contract.md",
        "09-ux-ui-spec.md",
        "10-frontend-architecture.md",
        "11-security-threat-model.md",
        "12-edge-cases.md",
        "13-implementation-plan.md",
        "14-e2e-test-matrix.md",
        "15-cross-ref.md",
        "16-documentation-plan.md",
        "lenses/LENS-product-owner.md",
        "lenses/LENS-full-stack.md",
        "lenses/LENS-database.md",
        "lenses/LENS-ux-ui.md",
        "lenses/LENS-front.md",
        "lenses/LENS-security.md",
        "lenses/LENS-keycloak.md",
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
    print(f"OK: SPEC-158 pack valid ({len(WARNINGS)} warning(s)). Root={ROOT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
