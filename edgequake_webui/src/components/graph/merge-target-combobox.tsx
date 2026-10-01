'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
    Command,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
} from '@/components/ui/command';
import {
    Popover,
    PopoverContent,
    PopoverTrigger,
} from '@/components/ui/popover';
import { cn } from '@/lib/utils';
import { Check, ChevronDown, GitMerge } from 'lucide-react';
import { useId, useMemo, useState } from 'react';

export interface MergeCandidate {
  id: string;
  label: string;
  entity_type: string;
}

interface MergeTargetComboboxProps {
  candidates: MergeCandidate[];
  value: string;
  onChange: (id: string) => void;
  currentLabel: string;
  currentType: string;
  disabled?: boolean;
}

function scoreCandidate(
  candidate: MergeCandidate,
  currentLabel: string,
  currentType: string,
): number {
  const label = candidate.label.toLowerCase();
  const current = currentLabel.toLowerCase();
  const currentTokens = current.split(/[^a-z0-9]+/).filter(Boolean);

  let score = 0;

  if (candidate.entity_type === currentType) score += 100;
  if (label === current) score += 80;
  if (label.startsWith(current) || current.startsWith(label)) score += 60;

  for (const token of currentTokens) {
    if (label.includes(token)) score += 20;
  }

  return score;
}

export function MergeTargetCombobox({
  candidates,
  value,
  onChange,
  currentLabel,
  currentType,
  disabled = false,
}: MergeTargetComboboxProps) {
  const [open, setOpen] = useState(false);
  const listboxId = useId();

  const rankedCandidates = useMemo(() => {
    return [...candidates].sort((a, b) => {
      const rankDiff =
        scoreCandidate(b, currentLabel, currentType) -
        scoreCandidate(a, currentLabel, currentType);

      if (rankDiff !== 0) return rankDiff;
      return a.label.localeCompare(b.label);
    });
  }, [candidates, currentLabel, currentType]);

  const selectedCandidate = rankedCandidates.find((candidate) => candidate.id === value);

  return (
    <div className="space-y-2">
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            id="merge-target-combobox"
            type="button"
            variant="outline"
            role="combobox"
            aria-expanded={open}
            aria-controls={listboxId}
            aria-label="Search and select target entity"
            data-testid="merge-target-combobox"
            disabled={disabled || rankedCandidates.length === 0}
            className="h-auto min-h-10 w-full justify-between px-3 py-2"
          >
            {selectedCandidate ? (
              <div className="flex min-w-0 items-center gap-2 text-left">
                <GitMerge className="h-4 w-4 shrink-0 text-primary" />
                <div className="min-w-0">
                  <div className="truncate text-sm font-medium">
                    {selectedCandidate.label}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {selectedCandidate.entity_type}
                  </div>
                </div>
              </div>
            ) : (
              <div className="flex min-w-0 items-center gap-2 text-left text-muted-foreground">
                <GitMerge className="h-4 w-4 shrink-0" />
                <span className="truncate">
                  Search and select an entity to merge into
                </span>
              </div>
            )}
            <ChevronDown className="ml-2 h-4 w-4 shrink-0 opacity-60" />
          </Button>
        </PopoverTrigger>
        <PopoverContent id={listboxId} className="w-90 p-0" align="start">
          <Command loop>
            <CommandInput
              placeholder="Search entities by name or type..."
              data-testid="merge-target-search"
            />
            <CommandList className="max-h-80">
              <CommandEmpty>No matching entities found.</CommandEmpty>
              <CommandGroup heading={`Available targets (${rankedCandidates.length})`}>
                {rankedCandidates.map((candidate) => {
                  const isRecommended = candidate.entity_type === currentType;
                  const isSelected = candidate.id === value;

                  return (
                    <CommandItem
                      key={candidate.id}
                      value={`${candidate.label} ${candidate.entity_type} ${candidate.id}`}
                      keywords={[candidate.entity_type, candidate.id]}
                      onSelect={() => {
                        onChange(candidate.id);
                        setOpen(false);
                      }}
                      className="items-start gap-2 py-2"
                    >
                      <Check
                        className={cn(
                          'mt-0.5 h-4 w-4 shrink-0',
                          isSelected ? 'opacity-100' : 'opacity-0'
                        )}
                      />
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2">
                          <span className="truncate text-sm font-medium">
                            {candidate.label}
                          </span>
                          <Badge variant="outline" className="text-xs">
                            {candidate.entity_type}
                          </Badge>
                          {isRecommended && (
                            <Badge variant="secondary" className="text-xs">
                              Recommended
                            </Badge>
                          )}
                        </div>
                      </div>
                    </CommandItem>
                  );
                })}
              </CommandGroup>
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>

      <p className="text-xs text-muted-foreground">
        Best matches are ranked first by similar name and entity type.
      </p>
    </div>
  );
}

export default MergeTargetCombobox;
