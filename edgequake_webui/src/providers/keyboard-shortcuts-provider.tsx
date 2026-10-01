'use client';

import { CommandPalette } from '@/components/shared/command-palette';
import { KeyboardShortcutsDialog } from '@/components/shared/keyboard-shortcuts-dialog';
import { useKeyboardShortcuts } from '@/hooks/use-keyboard-shortcuts';
import { createContext, useContext, type ReactNode } from 'react';

interface KeyboardShortcutsContextType {
  helpOpen: boolean;
  openHelp: () => void;
  closeHelp: () => void;
  searchOpen: boolean;
  openSearch: () => void;
  closeSearch: () => void;
}

const KeyboardShortcutsContext = createContext<KeyboardShortcutsContextType | null>(null);

export function useKeyboardShortcutsContext() {
  const context = useContext(KeyboardShortcutsContext);
  if (!context) {
    throw new Error('useKeyboardShortcutsContext must be used within a KeyboardShortcutsProvider');
  }
  return context;
}

interface KeyboardShortcutsProviderProps {
  children: ReactNode;
}

/**
 * Single mount for global keyboard shortcuts + Cmd+K palette (SPEC-155 F-155-S01/S02).
 */
export function KeyboardShortcutsProvider({ children }: KeyboardShortcutsProviderProps) {
  const {
    helpOpen,
    setHelpOpen,
    openHelp,
    closeHelp,
    searchOpen,
    openSearch,
    closeSearch,
  } = useKeyboardShortcuts();

  return (
    <KeyboardShortcutsContext.Provider
      value={{
        helpOpen,
        openHelp,
        closeHelp,
        searchOpen,
        openSearch,
        closeSearch,
      }}
    >
      {children}
      <KeyboardShortcutsDialog open={helpOpen} onOpenChange={setHelpOpen} />
      <CommandPalette />
    </KeyboardShortcutsContext.Provider>
  );
}

export default KeyboardShortcutsProvider;
