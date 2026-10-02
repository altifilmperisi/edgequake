/**
 * SPEC-091 IS0–IS3 — Ingestion surface: counts SSOT, queue chrome, phase strip, fence.
 *
 * Covers pdf / markdown / text / image ActiveRuns chrome with mocked list API.
 */
import { expect, test, type Page } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";
import {
  makeSpec086ListDoc,
  mockSpec086BusyPipeline,
  mockSpec086DocumentList,
  type Spec086ListDoc,
} from "./helpers/spec086-ingestion-mocks";
import { expandIntakeWorking } from "./helpers/workspace-runs";

type Counts = { unit: string; current: number; total: number };

function withCounts(
  doc: Spec086ListDoc,
  progress_counts: Counts,
): Spec086ListDoc & { progress_counts: Counts } {
  return { ...doc, progress_counts };
}

async function gotoDocuments(page: Page, docs: Spec086ListDoc[]) {
  // Match SPEC-086 order: admission routes → seed → list/pipeline mocks (last wins).
  await mockSpec038AdmissionRoutes(page);
  await seedSpec038TenantContext(page);
  await mockSpec086BusyPipeline(page);
  await mockSpec086DocumentList(page, docs);
  await page.goto("/documents", GOTO_OPTS);
  await expandIntakeWorking(page);
  await expect(page.getByTestId("spec048-active-runs-panel")).toBeVisible({
    timeout: 20_000,
  });
}

test.describe("SPEC-091 ingestion surface (progress_counts + single meter)", () => {
  test("pdf converting: structured counts → one stage meter, no nested page bar", async ({
    page,
  }) => {
    const doc = withCounts(
      makeSpec086ListDoc({
        id: "doc-091-pdf",
        file_name: "01-databricks-ticket.pdf",
        status: "processing",
        current_stage: "converting",
        stage_message: "Converting PDF to Markdown",
        stage_progress: 0.44,
        source_type: "pdf",
        track_id: "track-091-pdf",
        admission_staging: false,
      }),
      { unit: "pages", current: 4, total: 9 },
    );
    await gotoDocuments(page, [doc]);

    const card = page.getByTestId("spec048-active-run-card").first();
    await expect(card).toBeVisible();
    await expect(card.getByTestId("spec048-stage-progress")).toBeVisible();
    await expect(card.getByTestId("spec048-overall-progress")).toHaveAttribute(
      "data-collapsed",
      "true",
    );
    await expect(page.getByTestId("spec086-pdf-page-detail")).toHaveCount(0);
    // LAW-IS3: live row has no stage subtitle under ActiveRuns.
    await expect(page.getByTestId("spec048-row-stage")).toHaveCount(0);
    // IS3: 4-phase strip; Prepare active for converting.
    await expect(card.getByTestId("spec091-phase-strip")).toBeVisible();
    await expect(card.getByTestId("spec091-phase-prepare")).toHaveAttribute(
      "data-state",
      "active",
    );
  });

  test("pdf converting with insert-* track: never nests PDF page meter", async ({
    page,
  }) => {
    const doc = makeSpec086ListDoc({
      id: "doc-091-insert-track",
      file_name: "paper.pdf",
      status: "processing",
      current_stage: "converting",
      stage_message:
        "Processed 22 chunks, extracted 658 entities and 381 relationships",
      stage_progress: 0,
      source_type: "pdf",
      track_id: "insert-f1d3c215-2042-4a33-b284-518a90e020ff",
      admission_staging: false,
    });
    await gotoDocuments(page, [doc]);

    const card = page.getByTestId("spec048-active-run-card").first();
    await expect(card).toBeVisible();
    await expect(page.getByTestId("spec086-pdf-page-detail")).toHaveCount(0);
    await expect(page.getByTestId("spec086-pdf-progress-ended")).toHaveCount(0);
    await expect(
      page.getByText(/Task ended — progress is no longer available/i),
    ).toHaveCount(0);
  });

  test("markdown chunking: converting omitted; counts drive stage meter", async ({
    page,
  }) => {
    const doc = withCounts(
      makeSpec086ListDoc({
        id: "doc-091-md",
        file_name: "notes.md",
        status: "processing",
        current_stage: "chunking",
        stage_message: "Chunking document",
        stage_progress: 0.33,
        source_type: "markdown",
        track_id: "track-091-md",
        admission_staging: false,
      }),
      { unit: "chunks", current: 1, total: 3 },
    );
    await gotoDocuments(page, [doc]);

    const card = page.getByTestId("spec048-active-run-card").first();
    await expect(card.getByTestId("spec048-stage-chunking")).toHaveAttribute(
      "data-state",
      "active",
    );
    await expect(card.getByTestId("spec048-stage-converting")).toHaveCount(0);
    await expect(card.getByTestId("spec048-stage-progress")).toBeVisible();
    await expect(page.getByTestId("spec086-pdf-page-detail")).toHaveCount(0);
  });

  test("text extracting: no converting chip; structured entity/chunk counts", async ({
    page,
  }) => {
    const doc = withCounts(
      makeSpec086ListDoc({
        id: "doc-091-txt",
        file_name: "readme.txt",
        status: "processing",
        current_stage: "extracting",
        stage_message: "Extracting entities and relationships…",
        stage_progress: 0.1,
        source_type: "text",
        track_id: "track-091-txt",
        admission_staging: false,
      }),
      { unit: "chunks", current: 2, total: 20 },
    );
    await gotoDocuments(page, [doc]);

    const card = page.getByTestId("spec048-active-run-card").first();
    await expect(card.getByTestId("spec048-run-headline")).toContainText("2/20");
    await expect(card.getByTestId("spec048-stage-converting")).toHaveCount(0);
    await expect(card.getByTestId("spec091-phase-extract")).toHaveAttribute(
      "data-state",
      "active",
    );
  });

  test("image: converting omitted like non-PDF sources", async ({ page }) => {
    const doc = withCounts(
      makeSpec086ListDoc({
        id: "doc-091-img",
        file_name: "shot.png",
        status: "processing",
        current_stage: "extracting",
        stage_message: "Extracting entities — chunk 1/1",
        stage_progress: 0.5,
        source_type: "image",
        track_id: "track-091-img",
        admission_staging: false,
      }),
      { unit: "chunks", current: 1, total: 1 },
    );
    await gotoDocuments(page, [doc]);

    const card = page.getByTestId("spec048-active-run-card").first();
    await expect(card.getByTestId("spec048-stage-converting")).toHaveCount(0);
    await expect(card.getByTestId("spec048-stage-progress")).toBeVisible();
  });
});

test.describe("SPEC-091 IS2–IS3 queue / phase / fence", () => {
  test("IS-AC-04: queued shows position + ETA; never Extracting@0% sole status", async ({
    page,
  }) => {
    const doc = makeSpec086ListDoc({
      id: "doc-091-queued",
      file_name: "wait.pdf",
      status: "pending",
      current_stage: "queued",
      stage_message: "Waiting for a free worker…",
      stage_progress: 0,
      source_type: "pdf",
      track_id: "track-091-queued",
      admission_staging: false,
      queue_position: 3,
      eta_seconds: 120,
      eta_basis: "measured",
    });
    await gotoDocuments(page, [doc]);

    const card = page.getByTestId("spec048-active-run-card").first();
    await expect(card.getByTestId("spec048-run-headline")).toContainText("Queued");
    await expect(card.getByTestId("spec048-run-headline")).toContainText("#3");
    const runMessage = card.getByTestId("spec086-run-message");
    if (await runMessage.count()) {
      await expect(runMessage).toContainText(/#3|queued/i);
    }
    await expect(card.getByTestId("spec048-run-headline")).not.toContainText(
      "Extracting",
    );
    await expect(card.getByTestId("spec091-phase-admit")).toHaveAttribute(
      "data-state",
      "active",
    );
  });

  test("IS-AC-06/07: phase strip + Working·Queued header when both matter", async ({
    page,
  }) => {
    const working = withCounts(
      makeSpec086ListDoc({
        id: "doc-091-working",
        file_name: "live.pdf",
        status: "processing",
        current_stage: "gleaning",
        stage_message: "Refining entities",
        stage_progress: 0.4,
        source_type: "pdf",
        track_id: "track-091-working",
        admission_staging: false,
        cost_usd: 0.12,
      }),
      { unit: "chunks", current: 4, total: 10 },
    );
    const queued = makeSpec086ListDoc({
      id: "doc-091-q2",
      file_name: "later.md",
      status: "pending",
      current_stage: "queued",
      stage_message: "Waiting…",
      stage_progress: 0,
      source_type: "markdown",
      track_id: "track-091-q2",
      admission_staging: false,
      queue_position: 1,
      eta_seconds: 30,
      eta_basis: "measured",
    });
    await gotoDocuments(page, [working, queued]);

    await expect(
      page.getByTestId("pipeline-header-button"),
    ).toContainText(/Working/);
    await expect(
      page.getByTestId("pipeline-header-button"),
    ).toContainText(/Queued/);

    const gleanCard = page
      .locator('[data-testid="spec048-active-run-card"][data-stage="gleaning"]')
      .first();
    await expect(gleanCard.getByTestId("spec091-phase-extract")).toHaveAttribute(
      "data-state",
      "active",
    );
    await expect(gleanCard.getByTestId("spec048-run-headline")).toContainText(
      /Refining|Extract/i,
    );
    await expect(gleanCard.getByTestId("spec091-run-cost")).toContainText(
      "$0.12",
    );
  });

  test("IS-AC-07 fence: Ready vs Indexed when query_ready set", async ({
    page,
  }) => {
    const ready = makeSpec086ListDoc({
      id: "doc-091-ready",
      file_name: "ready.pdf",
      status: "completed",
      current_stage: "completed",
      stage_message: "Completed",
      stage_progress: 1,
      source_type: "pdf",
      track_id: null,
      admission_staging: false,
      query_ready: true,
    });
    const indexed = makeSpec086ListDoc({
      id: "doc-091-indexed",
      file_name: "pending-query.pdf",
      status: "completed",
      current_stage: "completed",
      stage_message: "Completed",
      stage_progress: 1,
      source_type: "pdf",
      track_id: null,
      admission_staging: false,
      query_ready: false,
    });
    // Completed rows live in the table (not ActiveRuns) — admission + list mocks only.
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);
    await mockSpec086DocumentList(page, [ready, indexed]);
    await page.goto("/documents", GOTO_OPTS);
    await expect(page.getByText("ready.pdf").first()).toBeVisible({
      timeout: 20_000,
    });
    await expect(
      page.locator('[data-testid="spec091-serving-fence-badge"][data-query-ready="true"]'),
    ).toBeVisible({ timeout: 20_000 });
    await expect(
      page.locator(
        '[data-testid="spec091-serving-fence-badge"][data-query-ready="false"]',
      ),
    ).toContainText("not queryable");
  });
});
