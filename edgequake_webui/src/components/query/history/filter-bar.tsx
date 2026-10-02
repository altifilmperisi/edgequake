"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useConversationFilters } from "@/stores/use-query-ui-store";
import { Archive, Pin, X } from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

interface FilterBarProps {
  onClose: () => void;
}

export function HistoryFilterBar({ onClose }: FilterBarProps) {
  const { t } = useTranslation();
  const { filters, setFilters, resetFilters } = useConversationFilters();

  const hasActiveFilters = useMemo(() => {
    return (
      filters.pinned !== null ||
      filters.archived ||
      (filters.mode && filters.mode.length > 0) ||
      filters.dateFrom ||
      filters.dateTo
    );
  }, [filters]);

  return (
    <div className="border-b bg-muted/20 p-2 space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium">{t("query.filters", "Filters")}</span>
        <div className="flex items-center gap-1">
          {hasActiveFilters ? (
            <Button
              variant="ghost"
              size="sm"
              className="h-5 px-1.5 text-xs"
              onClick={resetFilters}
            >
              {t("common.clear", "Clear")}
            </Button>
          ) : null}
          <Button variant="ghost" size="icon" className="h-5 w-5" onClick={onClose}>
            <X className="h-3 w-3" />
          </Button>
        </div>
      </div>
      <div className="flex flex-wrap gap-1.5">
        <Badge
          variant={filters.pinned === true ? "default" : "outline"}
          className="cursor-pointer text-xs px-2 py-0.5"
          onClick={() =>
            setFilters({ pinned: filters.pinned === true ? null : true })
          }
        >
          <Pin className="h-2.5 w-2.5 mr-1" />
          {t("query.pinned", "Pinned")}
        </Badge>
        <Badge
          variant={filters.archived ? "default" : "outline"}
          className="cursor-pointer text-xs px-2 py-0.5"
          onClick={() => setFilters({ archived: !filters.archived })}
        >
          <Archive className="h-2.5 w-2.5 mr-1" />
          {t("query.archived", "Archived")}
        </Badge>
      </div>
    </div>
  );
}
