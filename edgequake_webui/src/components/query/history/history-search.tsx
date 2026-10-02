"use client";

import { Input } from "@/components/ui/input";
import { useConversationFilters } from "@/stores/use-query-ui-store";
import { Search } from "lucide-react";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";

export function HistorySearch() {
  const { t } = useTranslation();
  const { filters, setFilters } = useConversationFilters();

  const handleSearchChange = useCallback(
    (value: string) => {
      setFilters({ search: value });
    },
    [setFilters],
  );

  return (
    <div className="p-2 border-b shrink-0">
      <div className="relative">
        <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground" />
        <Input
          placeholder={t("query.history.search", "Search conversations...")}
          value={filters.search}
          onChange={(e) => handleSearchChange(e.target.value)}
          className="h-7 pl-7 text-xs bg-muted/30 border-muted focus:bg-background transition-colors"
          data-testid="query-history-search"
          aria-label={t("query.history.search", "Search conversations...")}
        />
      </div>
    </div>
  );
}
