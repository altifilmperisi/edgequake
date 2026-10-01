'use client';

import { Button } from '@/components/ui/button';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
    Tooltip,
    TooltipContent,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { useSettingsStore } from '@/stores/use-settings-store';
import { LayoutGrid, Loader2 } from 'lucide-react';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import type { GraphLayoutType } from '@/lib/graph/layouts';

export function LayoutControl() {
  const { t } = useTranslation();
  const setGraphSettings = useSettingsStore((s) => s.setGraphSettings);
  const currentLayout = useSettingsStore((s) => s.graphSettings.layout ?? 'force');
  const [isApplying, setIsApplying] = useState(false);

  const applyLayout = useCallback(
    async (layout: GraphLayoutType) => {
      setIsApplying(true);

      try {
        setGraphSettings({ layout });
        toast.success(`Applied ${layout} layout`);
      } catch (error) {
        console.error('Layout failed:', error);
        toast.error('Failed to apply layout');
      } finally {
        setIsApplying(false);
      }
    },
    [setGraphSettings]
  );

  return (
    <DropdownMenu>
      <Tooltip>
        <TooltipTrigger asChild>
          <DropdownMenuTrigger asChild>
            <Button 
              variant="ghost" 
              size="icon" 
              aria-label={t('graph.layouts.title', 'Change layout')}
              disabled={isApplying}
            >
              {isApplying ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <LayoutGrid className="h-4 w-4" />
              )}
            </Button>
          </DropdownMenuTrigger>
        </TooltipTrigger>
        <TooltipContent side="bottom">
          <div className="space-y-1">
            <div className="font-medium text-xs">{t('graph.layouts.title', 'Graph Layout')}</div>
            <p className="text-xs text-muted-foreground">Rearrange nodes with different algorithms</p>
          </div>
        </TooltipContent>
      </Tooltip>
      <DropdownMenuContent>
        <DropdownMenuItem 
          onClick={() => applyLayout('force')}
          className={currentLayout === 'force' ? 'bg-accent' : ''}
        >
          ⚡ {t('graph.layouts.force', 'Force Atlas')}
        </DropdownMenuItem>
        <DropdownMenuItem 
          onClick={() => applyLayout('force-directed')}
          className={currentLayout === 'force-directed' ? 'bg-accent' : ''}
        >
          🔄 {t('graph.layouts.forceDirected', 'Force Directed')}
        </DropdownMenuItem>
        <DropdownMenuItem 
          onClick={() => applyLayout('circular')}
          className={currentLayout === 'circular' ? 'bg-accent' : ''}
        >
          ⭕ {t('graph.layouts.circular', 'Circular')}
        </DropdownMenuItem>
        <DropdownMenuItem 
          onClick={() => applyLayout('random')}
          className={currentLayout === 'random' ? 'bg-accent' : ''}
        >
          🎲 {t('graph.layouts.random', 'Random')}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem 
          onClick={() => applyLayout('noverlaps')}
          className={currentLayout === 'noverlaps' ? 'bg-accent' : ''}
        >
          📐 {t('graph.layouts.noverlap', 'No Overlap')}
        </DropdownMenuItem>
        <DropdownMenuItem 
          onClick={() => applyLayout('circlepack')}
          className={currentLayout === 'circlepack' ? 'bg-accent' : ''}
        >
          🎯 {t('graph.layouts.circlepack', 'Circle Pack')}
        </DropdownMenuItem>
        <DropdownMenuItem 
          onClick={() => applyLayout('hierarchical')}
          className={currentLayout === 'hierarchical' ? 'bg-accent' : ''}
        >
          🌳 {t('graph.layouts.hierarchical', 'Hierarchical')}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export default LayoutControl;
