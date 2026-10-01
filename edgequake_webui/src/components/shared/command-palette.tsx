/**
 * Global Cmd+K command palette — SPEC-155 LAW-155-6 / F-155-S01.
 */
'use client';

import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from '@/components/ui/command';
import { useKeyboardShortcutsContext } from '@/providers/keyboard-shortcuts-provider';
import {
  FileText,
  GitBranch,
  Home,
  MessageSquare,
  Settings,
  Wallet,
} from 'lucide-react';
import { useTheme } from 'next-themes';
import { useRouter } from 'next/navigation';
import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';

const ROUTES = [
  { href: '/', icon: Home, labelKey: 'nav.home', fallback: 'Dashboard' },
  { href: '/documents', icon: FileText, labelKey: 'nav.documents', fallback: 'Documents' },
  { href: '/query', icon: MessageSquare, labelKey: 'nav.query', fallback: 'Query' },
  { href: '/graph', icon: GitBranch, labelKey: 'nav.graph', fallback: 'Knowledge Graph' },
  { href: '/pipeline', icon: GitBranch, labelKey: 'nav.pipeline', fallback: 'Pipeline' },
  { href: '/costs', icon: Wallet, labelKey: 'nav.costs', fallback: 'Costs' },
  { href: '/settings', icon: Settings, labelKey: 'nav.settings', fallback: 'Settings' },
] as const;

export function CommandPalette() {
  const { t } = useTranslation();
  const router = useRouter();
  const { searchOpen, closeSearch, openHelp } = useKeyboardShortcutsContext();
  const { setTheme } = useTheme();

  const go = useCallback(
    (href: string) => {
      closeSearch();
      router.push(href);
    },
    [closeSearch, router],
  );

  return (
    <CommandDialog open={searchOpen} onOpenChange={(open) => (!open ? closeSearch() : undefined)}>
      <CommandInput data-testid="command-palette-input" placeholder={t('command.placeholder', 'Search pages and actions…')} />
      <CommandList>
        <CommandEmpty>{t('command.empty', 'No results')}</CommandEmpty>
        <CommandGroup heading={t('command.navigate', 'Navigate')}>
          {ROUTES.map((r) => (
            <CommandItem key={r.href} onSelect={() => go(r.href)}>
              <r.icon className="mr-2 h-4 w-4" />
              {t(r.labelKey, r.fallback)}
            </CommandItem>
          ))}
        </CommandGroup>
        <CommandSeparator />
        <CommandGroup heading={t('command.actions', 'Actions')}>
          <CommandItem
            onSelect={() => {
              setTheme('light');
              closeSearch();
            }}
          >
            {t('theme.light', 'Light theme')}
          </CommandItem>
          <CommandItem
            onSelect={() => {
              setTheme('dark');
              closeSearch();
            }}
          >
            {t('theme.dark', 'Dark theme')}
          </CommandItem>
          <CommandItem
            onSelect={() => {
              closeSearch();
              openHelp();
            }}
          >
            {t('command.shortcuts', 'Keyboard shortcuts')}
          </CommandItem>
        </CommandGroup>
      </CommandList>
    </CommandDialog>
  );
}
