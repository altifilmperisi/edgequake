'use client';

/**
 * @module NodeContextMenu
 * @description Context menu for a graph node, opened by a canvas right-click at
 * the cursor position.
 *
 * Built on the Radix `DropdownMenu` primitive (DRY with the rest of the app) so
 * keyboard navigation, focus management, Escape-to-close, and click-outside are
 * inherited rather than hand-rolled:
 *  - ArrowUp/Down + Home/End + type-ahead move between items.
 *  - Enter/Space selects; Escape closes; focus is trapped while open.
 *  - Viewport collision (flip/clamp) is handled by Radix Popper — no manual
 *    coordinate clamping.
 *
 * Lifecycle (the pointer side lives in lib/graph/engine/pointer-fsm.ts):
 *
 *   CLOSED ── right-click node / Ctrl+click / Menu key / Shift+F10 ──► OPEN(node)
 *   OPEN ── Esc | click outside | item chosen ──► CLOSED (focus → canvas)
 *   OPEN: target node stays emphasised, selection is untouched, the canvas is
 *         frozen (no hover, no drag); closing replays the pointer for hover.
 *   Delete is a nested state: OPEN ──"Delete Entity"──► CONFIRM ──► CLOSED | OPEN.
 *
 * Because the menu is opened programmatically from a canvas (not a DOM trigger),
 * a 1px `Anchor` is placed at the cursor to position the content.
 */

import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
  } from '@/components/ui/dropdown-menu';
import { useEntityTypeColors } from '@/hooks/use-entity-type-colors';
import { formatEntityLabel, formatEntityType } from '@/lib/graph/label-utils';
import type { GraphNode } from '@/types';
import * as DropdownMenuPrimitive from '@radix-ui/react-dropdown-menu';
import {
    Check,
    Copy,
    Eye,
    FileText,
    Minimize2,
    Network,
    Search,
    Trash2
} from 'lucide-react';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';

interface NodeContextMenuPosition {
  x: number;
  y: number;
}

interface NodeContextMenuProps {
  node: GraphNode | null;
  position: NodeContextMenuPosition | null;
  onClose: () => void;
  onViewDetails: (node: GraphNode) => void;
  onExpandNeighborhood: (node: GraphNode) => void;
  onPruneNode?: (node: GraphNode) => void;
  onFindRelated: (node: GraphNode) => void;
  onViewDocuments: (node: GraphNode) => void;
  onCopyId: (node: GraphNode) => void;
  onDelete?: (node: GraphNode) => void;
  isExpanded?: boolean;
}

export function NodeContextMenu({
  node,
  position,
  onClose,
  onViewDetails,
  onExpandNeighborhood,
  onPruneNode,
  onFindRelated,
  onViewDocuments,
  onCopyId,
  onDelete,
  isExpanded = false,
}: NodeContextMenuProps) {
  const { t } = useTranslation();
  // Two-step delete confirm — avoids accidental deletion without a full modal
  const [pendingDelete, setPendingDelete] = useState(false);

  const open = Boolean(node && position);

  const handleOpenChange = useCallback(
    (next: boolean) => {
      if (!next) {
        setPendingDelete(false);
        onClose();
      }
    },
    [onClose],
  );

  const { colorFor } = useEntityTypeColors();

  if (!node || !position) return null;

  const displayLabel = formatEntityLabel(node.label ?? '', 40);
  const displayType = formatEntityType(node.node_type ?? '');
  const typeColor = colorFor(node.node_type);

  return (
    <DropdownMenu open={open} onOpenChange={handleOpenChange}>
      {/*
        Virtual trigger at the right-click cursor. The menu is opened from the
        graph canvas, so there is no real DOM button; Radix positions the content
        against this 1px point and flips/clamps it within the viewport. It is
        pointer-events:none so it never intercepts interaction, and `open` is
        controlled externally so the trigger's built-in toggle never fires.
      */}
      <DropdownMenuPrimitive.Trigger asChild>
        <span
          aria-hidden
          tabIndex={-1}
          style={{
            position: 'fixed',
            left: position.x,
            top: position.y,
            width: 1,
            height: 1,
            pointerEvents: 'none',
          }}
        />
      </DropdownMenuPrimitive.Trigger>

      <DropdownMenuContent
        side="right"
        align="start"
        sideOffset={8}
        collisionPadding={8}
        className="min-w-60"
        data-testid="node-context-menu"
        // Focus returns to the canvas (not the 1px anchor / body) so the
        // keyboard shortcuts keep working right after the menu closes.
        onCloseAutoFocus={(event) => {
          event.preventDefault();
          document
            .querySelector<HTMLElement>("[data-graph-engine]")
            ?.focus({ preventScroll: true });
        }}
      >
        {/* Header: formatted name + type with color dot */}
        <DropdownMenuLabel className="font-normal" data-testid="node-context-menu-header">
          <div
            className="font-semibold text-sm truncate"
            title={formatEntityLabel(node.label ?? '', 200)}
          >
            {displayLabel}
          </div>
          <div className="flex items-center gap-1.5 mt-0.5">
            <span
              className="inline-block w-2 h-2 rounded-full shrink-0"
              style={{ backgroundColor: typeColor }}
              aria-hidden="true"
            />
            <span className="text-xs text-muted-foreground">{displayType}</span>
          </div>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />

        <DropdownMenuItem onSelect={() => onViewDetails(node)}>
          <Eye />
          <span className="flex-1">{t('graph.contextMenu.viewDetails', 'View Details')}</span>
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={() => onExpandNeighborhood(node)}>
          <Network />
          <span className="flex-1">{t('graph.contextMenu.expandNeighborhood', 'Expand Neighborhood')}</span>
          {isExpanded && <Check className="h-3.5 w-3.5 text-muted-foreground" aria-hidden="true" />}
        </DropdownMenuItem>
        {onPruneNode && (
          <DropdownMenuItem onSelect={() => onPruneNode(node)}>
            <Minimize2 />
            <span className="flex-1">{t('graph.contextMenu.pruneNode', 'Prune Node')}</span>
          </DropdownMenuItem>
        )}
        <DropdownMenuItem onSelect={() => onFindRelated(node)}>
          <Search />
          <span className="flex-1">{t('graph.contextMenu.findRelated', 'Find Related')}</span>
        </DropdownMenuItem>

        <DropdownMenuSeparator />

        <DropdownMenuItem onSelect={() => onViewDocuments(node)}>
          <FileText />
          <span className="flex-1">{t('graph.contextMenu.viewDocuments', 'View Documents')}</span>
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={() => onCopyId(node)}>
          <Copy />
          <span className="flex-1">{t('graph.contextMenu.copyId', 'Copy Entity ID')}</span>
        </DropdownMenuItem>

        {onDelete && (
          <>
            <DropdownMenuSeparator />
            {pendingDelete ? (
              <>
                <DropdownMenuLabel className="text-destructive text-xs font-medium">
                  {t('graph.contextMenu.deleteConfirm', 'Delete this entity?')}
                </DropdownMenuLabel>
                <DropdownMenuItem
                  // Keep the menu open so the user can confirm or cancel.
                  onSelect={(event) => {
                    event.preventDefault();
                    setPendingDelete(false);
                  }}
                >
                  <span className="flex-1">{t('common.cancel', 'Cancel')}</span>
                </DropdownMenuItem>
                <DropdownMenuItem
                  variant="destructive"
                  data-testid="node-context-menu-delete-confirm"
                  onSelect={() => onDelete(node)}
                >
                  <Trash2 />
                  <span className="flex-1">{t('common.delete', 'Delete')}</span>
                </DropdownMenuItem>
              </>
            ) : (
              <DropdownMenuItem
                variant="destructive"
                // Keep the menu open to reveal the inline confirm step.
                onSelect={(event) => {
                  event.preventDefault();
                  setPendingDelete(true);
                }}
              >
                <Trash2 />
                <span className="flex-1">{t('graph.contextMenu.deleteEntity', 'Delete Entity')}</span>
              </DropdownMenuItem>
            )}
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/**
 * Hook to manage context-menu open state. Open/close side effects (Escape,
 * click-outside) are handled by the Radix DropdownMenu via `onClose`, so this
 * hook only tracks which node + cursor position the menu is for.
 */
export function useNodeContextMenu() {
  const [contextMenuState, setContextMenuState] = useState<{
    node: GraphNode | null;
    position: { x: number; y: number } | null;
  }>({ node: null, position: null });

  const openContextMenu = useCallback((node: GraphNode, x: number, y: number) => {
    setContextMenuState({ node, position: { x, y } });
  }, []);

  const closeContextMenu = useCallback(() => {
    setContextMenuState({ node: null, position: null });
  }, []);

  return {
    contextMenuNode: contextMenuState.node,
    contextMenuPosition: contextMenuState.position,
    openContextMenu,
    closeContextMenu,
  };
}

export default NodeContextMenu;
