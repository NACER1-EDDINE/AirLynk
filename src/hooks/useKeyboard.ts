import { useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

export interface KeyboardHandlers {
  openFilePicker: () => Promise<void>;
  onKeyDown: (event: KeyboardEvent) => void;
}

export function useKeyboard(
  onOpenFilePicker?: () => void,
  onEscape?: () => void,
  onTab?: (shiftKey: boolean) => void
): KeyboardHandlers {
  const openFilePicker = useCallback(async () => {
    try {
      await invoke('pick_files');
    } catch {
      onOpenFilePicker?.();
    }
  }, [onOpenFilePicker]);

  const onKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        onEscape?.();
        return;
      }

      if (event.key === 'Tab') {
        event.preventDefault();
        onTab?.(event.shiftKey);
        return;
      }

      if ((event.ctrlKey || event.metaKey) && event.key === 'o') {
        event.preventDefault();
        openFilePicker();
        return;
      }
    },
    [openFilePicker, onEscape, onTab]
  );

  useEffect(() => {
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [onKeyDown]);

  return { openFilePicker, onKeyDown };
}