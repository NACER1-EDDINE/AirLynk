import { useState, useEffect, useCallback } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

export interface DragDropResult {
  isDragging: boolean;
  onFilesDropped: (callback: (files: File[]) => void) => void;
  clearFiles: () => void;
}

export function useDragDrop(onFilesDropped?: (files: File[]) => void): DragDropResult {
  const [isDragging, setIsDragging] = useState(false);

  useEffect(() => {
    let unlistenDragover: UnlistenFn | undefined;
    let unlistenDrop: UnlistenFn | undefined;
    let unlistenDragLeave: UnlistenFn | undefined;

    const handleDropFiles = (fileList: FileList | File[] | null | undefined) => {
      const files = Array.from(fileList ?? []);
      if (files.length === 0) return;
      setIsDragging(false);
      onFilesDropped?.(files);
    };

    const handleWindowDragOver = (event: DragEvent) => {
      if (event.dataTransfer && event.dataTransfer.types.includes('Files')) {
        event.preventDefault();
        setIsDragging(true);
      }
    };

    const handleWindowDrop = (event: DragEvent) => {
      if (event.dataTransfer && event.dataTransfer.files.length > 0) {
        event.preventDefault();
        event.stopPropagation();
        handleDropFiles(event.dataTransfer.files);
      }
    };

    const handleWindowDragLeave = (event: DragEvent) => {
      if (event.target === window || (event.relatedTarget === null && event.target === document.body)) {
        setIsDragging(false);
      }
    };

    const setup = async () => {
      unlistenDragover = await listen('tauri://file-drop-hover', () => {
        setIsDragging(true);
      });

      unlistenDrop = await listen('tauri://file-drop', (event: { payload: string[] }) => {
        const filePaths = event.payload ?? [];
        const fileObjects = filePaths.map((path) => {
          const name = path.split('\\').pop() || path.split('/').pop() || 'unknown';
          return new File([], name, { type: '' });
        });
        onFilesDropped?.(fileObjects);
      });

      unlistenDragLeave = await listen('tauri://file-drop-cancel', () => {
        setIsDragging(false);
      });
    };

    window.addEventListener('dragover', handleWindowDragOver);
    window.addEventListener('drop', handleWindowDrop);
    window.addEventListener('dragleave', handleWindowDragLeave);
    void setup();

    return () => {
      window.removeEventListener('dragover', handleWindowDragOver);
      window.removeEventListener('drop', handleWindowDrop);
      window.removeEventListener('dragleave', handleWindowDragLeave);
      unlistenDragover?.();
      unlistenDrop?.();
      unlistenDragLeave?.();
    };
  }, [onFilesDropped]);

  const clearFiles = useCallback(() => {
    setIsDragging(false);
  }, []);

  return {
    isDragging,
    onFilesDropped: (callback: (files: File[]) => void) => {
      onFilesDropped = callback;
    },
    clearFiles,
  };
}