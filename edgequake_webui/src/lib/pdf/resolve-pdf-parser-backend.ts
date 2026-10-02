/**
 * PDF parser backend resolution (SPEC-123) — mirrors Rust SSOT.
 *
 * Priority: Upload > Workspace > Tenant > Server env > Vision.
 * Auto is an explicit choice (never inferred from unset+Vision).
 *
 * @see edgequake-pdf `resolve_pdf_parser_choice`
 * @see edgequake-api `PdfUploadOptions::resolved_pdf_parser`
 */

import type { PdfParserBackend } from "@/types/graph";

/** Per-upload selector value on the Documents dropzone. */
export type UploadPdfParserChoice =
  | "default"
  | "vision"
  | "edgeparse"
  | "edgeparse-ocr"
  | "auto";

export type PdfParserResolutionSource =
  | "upload"
  | "workspace"
  | "tenant"
  | "server"
  | "default";

export interface PdfParserResolutionContext {
  /** Upload-level selector (`default` = inherit workspace/tenant/server). */
  uploadChoice: UploadPdfParserChoice;
  /** Workspace default from `workspace.pdf_parser_backend`. */
  workspaceBackend?: PdfParserBackend | null;
  /** Tenant default from `tenant.pdf_parser_backend` (SPEC-123). */
  tenantBackend?: PdfParserBackend | null;
  /** Optional server override (tests / health); else env + Vision default. */
  serverBackend?: PdfParserBackend;
}

export interface PdfParserResolution {
  /** Winning config choice (may be `auto`). */
  backend: PdfParserBackend;
  /** Runtime converter: vision | edgeparse | edgeparse-ocr (`auto` starts as vision). */
  runtimeBackend: "vision" | "edgeparse" | "edgeparse-ocr";
  source: PdfParserResolutionSource;
  /** True when Vision/EdgeParse/EdgeParseOcr is inviolable (not Auto). */
  isExplicit: boolean;
  /** SPEC-038 may try EdgeParse only when true. */
  allowsAutoRoute: boolean;
}

/** Server default from `EDGEQUAKE_PDF_PARSER_BACKEND` (NEXT_PUBLIC mirror for UI). */
export function getServerDefaultPdfParserBackend(): PdfParserBackend {
  const raw =
    process.env.NEXT_PUBLIC_EDGEQUAKE_PDF_PARSER_BACKEND?.trim().toLowerCase() ??
    "";
  if (raw === "edgeparse" || raw === "edge-parse" || raw === "edge_parse") {
    return "edgeparse";
  }
  if (
    raw === "edgeparse-ocr" ||
    raw === "edgeparse_ocr" ||
    raw === "edge-parse-ocr" ||
    raw === "edgeparseocr"
  ) {
    return "edgeparse-ocr";
  }
  if (raw === "auto") {
    return "auto";
  }
  return "vision";
}

/** Human label for a concrete backend / choice (never "Server Default"). */
export function pdfParserBackendDisplayName(
  backend: PdfParserBackend,
): string {
  if (backend === "edgeparse") return "EdgeParse";
  if (backend === "edgeparse-ocr") return "EdgeParse + OCR";
  if (backend === "auto") return "Auto";
  return "Vision";
}

/**
 * LAW-101-2 / LAW-123-3 — never-silent server default for PDF parser.
 * Example: `Server Default (Vision)`.
 */
export function formatServerDefaultPdfParserLabel(
  t: (key: string, defaultValue: string, options?: { value: string }) => string,
  serverBackend: PdfParserBackend = getServerDefaultPdfParserBackend(),
): string {
  const value = pdfParserBackendDisplayName(serverBackend);
  return t(
    "settings.pdfParser.serverDefaultWithValue",
    `Server Default (${value})`,
    { value },
  );
}

/**
 * Documents upload inherit option — never-silent workspace (or tenant/server) default.
 * Example: `Workspace Default (Vision)`.
 */
export function formatWorkspaceDefaultPdfParserLabel(
  t: (key: string, defaultValue: string, options?: { value: string }) => string,
  workspaceBackend?: PdfParserBackend | null,
  serverBackend?: PdfParserBackend,
  tenantBackend?: PdfParserBackend | null,
): string {
  const resolved = resolvePdfParserBackend({
    uploadChoice: "default",
    workspaceBackend,
    tenantBackend,
    serverBackend,
  });
  const value = pdfParserBackendDisplayName(resolved.backend);
  return t(
    "documents.upload.pdfParserDefaultWithValue",
    `Workspace Default (${value})`,
    { value },
  );
}

function finalizeChoice(
  choice: PdfParserBackend,
  source: PdfParserResolutionSource,
): PdfParserResolution {
  if (choice === "auto") {
    return {
      backend: "auto",
      runtimeBackend: "vision",
      source,
      isExplicit: false,
      allowsAutoRoute: true,
    };
  }
  return {
    backend: choice,
    runtimeBackend: choice,
    source,
    isExplicit: true,
    allowsAutoRoute: false,
  };
}

/**
 * Resolve effective PDF parser using Upload → Workspace → Tenant → Env → Vision.
 */
export function resolvePdfParserBackend(
  ctx: PdfParserResolutionContext,
): PdfParserResolution {
  if (ctx.uploadChoice !== "default") {
    return finalizeChoice(ctx.uploadChoice, "upload");
  }
  if (ctx.workspaceBackend) {
    return finalizeChoice(ctx.workspaceBackend, "workspace");
  }
  if (ctx.tenantBackend) {
    return finalizeChoice(ctx.tenantBackend, "tenant");
  }
  const server = ctx.serverBackend ?? getServerDefaultPdfParserBackend();
  const source: PdfParserResolutionSource =
    ctx.serverBackend !== undefined ||
    Boolean(process.env.NEXT_PUBLIC_EDGEQUAKE_PDF_PARSER_BACKEND?.trim())
      ? "server"
      : "default";
  // Server/default Vision is inviolable (LAW-123-3) — not Auto.
  return finalizeChoice(server, source === "server" ? "server" : "default");
}

/** True when the resolved choice is inviolable Vision (not Auto). */
export function resolvesToVisionParser(
  ctx: PdfParserResolutionContext,
): boolean {
  return resolvePdfParserBackend(ctx).backend === "vision";
}
