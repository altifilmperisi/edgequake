/**
 * @vitest-environment jsdom
 */
import { PageHeader } from "@/components/shared/page-header";
import { StatusBadge } from "@/components/shared/status-badge";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

describe("PageHeader", () => {
  it("renders title as h1", () => {
    render(<PageHeader title="Documents" description="Manage corpus" />);
    expect(screen.getByRole("heading", { level: 1, name: "Documents" })).toBeInTheDocument();
    expect(screen.getByText("Manage corpus")).toBeInTheDocument();
  });
});

describe("StatusBadge", () => {
  it("maps completed to success tone class", () => {
    render(<StatusBadge label="completed" />);
    const el = screen.getByText("completed");
    expect(el.className).toMatch(/success/);
  });
});
