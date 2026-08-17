import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { MotionConfig } from 'framer-motion';
import { UnlistenFn } from '@tauri-apps/api/event';
import { Sleeve } from './components';
import { FirstRunWindow } from './components/FirstRun/FirstRunWindow';
import { useDragDrop } from './hooks/useDragDrop';
import { useReducedMotion } from './hooks/useReducedMotion';
import { WidgetState, FailureCause, nextState } from './lib/stateMachine';
import {
  startSendSession,
  startReceiveSession,
  onProgress,
  onSessionEvent,
  onMenuAction,
  pickFiles,
  ProgressEvent,
} from './lib/ipc';
import {
  unsealedState,
  stagedState,
  sealedState,
  transferringState,
  deliveredState,
  receiveState,
  failureStates,
} from './lib/mockData';
import type { SessionFile } from './types';
import './App.css';

function generateQRCodeSvg(displayCode: string): string {
  const size = 180;
  const moduleCount = 25;
  const moduleSize = size / moduleCount;
  const data = displayCode + 'AIRLYNK_TRANSFER';
  let hash = 0;
  for (let i = 0; i < data.length; i++) {
    hash = ((hash << 5) - hash) + data.charCodeAt(i);
    hash |= 0;
  }
  const pattern = Array(moduleCount).fill(0).map(() => Array(moduleCount).fill(false));
  for (let y = 0; y < moduleCount; y++) {
    for (let x = 0; x < moduleCount; x++) {
      const val = (hash + x * 7 + y * 13) % 97;
      pattern[y][x] = val < 48;
    }
  }
  for (let y = 0; y < 7; y++) {
    for (let x = 0; x < 7; x++) {
      if ((x === 0 || x === 6 || y === 0 || y === 6) && !(x === 0 && y === 0) && !(x === 6 && y === 0) && !(x === 0 && y === 6)) {
        pattern[y][x] = true;
        pattern[y][moduleCount - 1 - x] = true;
        pattern[moduleCount - 1 - y][x] = true;
      }
    }
  }
  pattern[6][8] = true; pattern[8][6] = true; pattern[8][8] = true;
  let paths = '';
  for (let y = 0; y < moduleCount; y++) {
    let x = 0;
    while (x < moduleCount) {
      if (pattern[y][x]) {
        let w = 0;
        while (x + w < moduleCount && pattern[y][x + w]) w++;
        paths += `<rect x="${x * moduleSize}" y="${y * moduleSize}" width="${w * moduleSize}" height="${moduleSize}" />`;
        x += w;
      } else {
        x++;
      }
    }
  }
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}"><g fill="currentColor">${paths}</g></svg>`;
}

function formatSpeed(bytesPerSec: number): string {
  if (bytesPerSec < 1024) return `${bytesPerSec} B/s`;
  if (bytesPerSec < 1024 * 1024) return `${(bytesPerSec / 1024).toFixed(1)} MB/s`;
  return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} GB/s`;
}

function App() {
  const filePickerRef = useRef<HTMLInputElement | null>(null);
  const [showFirstRun, setShowFirstRun] = useState(true);
  const [sleeveState, setSleeveState] = useState<WidgetState>('unsealed');
  const [files, setFiles] = useState<SessionFile[]>([]);
  const [selectedSendFiles, setSelectedSendFiles] = useState<File[]>([]);
  const [displayCode, setDisplayCode] = useState<string>('');
  const [byteTotal, setByteTotal] = useState(0);
  const [activeFileId, setActiveFileId] = useState<number | undefined>(undefined);
  const [bandProgress, setBandProgress] = useState<{ current: number; total: number; speed?: string } | undefined>(undefined);
  const [failureCause, setFailureCause] = useState<FailureCause | undefined>(undefined);
  const [itemCount, setItemCount] = useState(0);
  const prefersReduced = useReducedMotion();
  const liveRegionRef = useRef<HTMLDivElement | null>(null);

  const triggerFilePicker = useCallback(() => {
    if (filePickerRef.current) {
      filePickerRef.current.click();
      return;
    }

    void pickFiles();
  }, []);

  const handleReceiveClick = useCallback(async () => {
    try {
      const result = await startReceiveSession();
      setDisplayCode(result.displayCode);
      setSleeveState((current) => nextState(current, { type: 'receive-click' }));
    } catch (error) {
      console.error('Failed to start receive session:', error);
    }
  }, []);

  const handleFirstRunComplete = useCallback((mode?: 'send' | 'receive') => {
    setShowFirstRun(false);
    if (mode === 'send') {
      triggerFilePicker();
    } else if (mode === 'receive') {
      void handleReceiveClick();
    }
  }, [handleReceiveClick, triggerFilePicker]);

  const beginSendFiles = useCallback(async (selectedFiles: File[]) => {
    if (selectedFiles.length === 0) return;

    const sessionFiles: SessionFile[] = Array.from(selectedFiles).map((file, index) => ({
      id: index + 1,
      originalName: file.name,
      size: file.size,
      sentBytes: 0,
      status: 'pending' as const,
    }));

    setSelectedSendFiles(selectedFiles);
    setFiles(sessionFiles);
    setByteTotal(selectedFiles.reduce((sum: number, f) => sum + f.size, 0));
    setItemCount(selectedFiles.length);
    setSleeveState((current) => nextState(current, { type: 'drop', payload: { files: selectedFiles } }));

    try {
      const result = await startSendSession(selectedFiles);
      setDisplayCode(result.displayCode);
      setSleeveState((current) => nextState(current, { type: 'session-opened', payload: { displayCode: result.displayCode } }));
    } catch (error) {
      console.error('Failed to start send session:', error);
    }
  }, []);

  const handleSendClick = useCallback(async () => {
    if (selectedSendFiles.length === 0) {
      triggerFilePicker();
      return;
    }

    await beginSendFiles(selectedSendFiles);
  }, [beginSendFiles, selectedSendFiles, triggerFilePicker]);

  const handleFlip = useCallback(() => {
    // Flip is handled by Sleeve component internally
  }, []);

  const handleClose = useCallback(() => {
    if (sleeveState === 'transferring' || sleeveState === 'receive') {
      // Cancel active transfer
    } else if (sleeveState === 'delivered' || sleeveState === 'sealed' || sleeveState === 'failure') {
      setSleeveState('unsealed');
      setFiles([]);
      setDisplayCode('');
      setByteTotal(0);
      setActiveFileId(undefined);
      setBandProgress(undefined);
      setFailureCause(undefined);
      setItemCount(0);
    } else if (sleeveState === 'staged') {
      setSleeveState('unsealed');
    }
  }, [sleeveState]);

  const handleRetry = useCallback(() => {
    if (sleeveState === 'failure') {
      setFailureCause(undefined);
      setSleeveState('unsealed');
      setFiles([]);
      setDisplayCode('');
      setByteTotal(0);
      setActiveFileId(undefined);
      setBandProgress(undefined);
      setItemCount(0);
    }
  }, [sleeveState]);

  const handleDismiss = useCallback(() => {
    if (sleeveState === 'failure') {
      setFailureCause(undefined);
      setSleeveState('unsealed');
      setFiles([]);
      setDisplayCode('');
      setByteTotal(0);
      setActiveFileId(undefined);
      setBandProgress(undefined);
      setItemCount(0);
    }
  }, [sleeveState]);

  useEffect(() => {
    if (process.env.NODE_ENV === 'development') {
      console.warn('[Dev] Tauri commands not available');
    }
  }, []);

  // Progress listener
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    onProgress((progress: ProgressEvent) => {
      if (progress.token !== displayCode) return;
      const totalSent = progress.files.reduce((sum: number, f) => sum + f.sentBytes, 0);
      const totalSize = progress.totalBytes;
      const activeFile = progress.files.find(f => f.status === 'active');
      setBandProgress({
        current: totalSent,
        total: totalSize,
        speed: formatSpeed(progress.transferSpeed),
      });
      if (activeFile) {
        setActiveFileId(activeFile.id);
      }
      setFiles(prev => prev.map(f => {
        const updated = progress.files.find(pf => pf.id === f.id);
        if (!updated) return f;
        return {
          ...f,
          sentBytes: updated.sentBytes,
          status: updated.status,
          progress: updated.sentBytes > 0 && f.size > 0 ? Math.round((updated.sentBytes / f.size) * 100) : 0,
        };
      }));
    }).then(fn => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, [displayCode]);

  // Session event listeners
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    onSessionEvent('session-opened', (data) => {
      if (data.token === displayCode) {
        if (sleeveState === 'sealed' || sleeveState === 'receive') {
          setSleeveState('transferring');
        }
      }
    }).then(fn => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, [displayCode, sleeveState]);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    onSessionEvent('session-delivered', (data) => {
      if (data.token === displayCode) {
        setSleeveState('delivered');
      }
    }).then(fn => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, [displayCode]);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    onSessionEvent('session-error', (data) => {
      if (data.token === displayCode) {
        const cause = data.cause as FailureCause;
        setFailureCause(cause);
        setSleeveState('failure');
      }
    }).then(fn => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, [displayCode]);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    onSessionEvent('session-cancelled', (data) => {
      if (data.token === displayCode) {
        setSleeveState('unsealed');
        setFiles([]);
        setDisplayCode('');
        setByteTotal(0);
          setActiveFileId(undefined);
        setBandProgress(undefined);
        setFailureCause(undefined);
        setItemCount(0);
      }
    }).then(fn => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, [displayCode]);

  // Keyboard handlers per DESIGN.md §8.2
  const handleGlobalKeyDown = useCallback((e: KeyboardEvent) => {
    if (showFirstRun) return;

    // Don't intercept if user is typing in an input
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
      return;
    }

    switch (e.key) {
      case 's':
      case 'S':
        if (sleeveState === 'unsealed') {
          e.preventDefault();
          handleSendClick();
        }
        break;
      case 'r':
      case 'R':
        if (sleeveState === 'unsealed') {
          e.preventDefault();
          handleReceiveClick();
        }
        break;
      case 'Enter':
        if (sleeveState === 'staged') {
          e.preventDefault();
          handleSendClick();
        } else if (sleeveState === 'delivered') {
          e.preventDefault();
          handleClose();
        } else if (sleeveState === 'failure') {
          e.preventDefault();
          handleRetry();
        }
        break;
      case 'Escape':
        if (sleeveState === 'transferring' || sleeveState === 'receive') {
          e.preventDefault();
          handleClose();
        } else if (sleeveState === 'failure') {
          e.preventDefault();
          handleDismiss();
        }
        break;
      case 'f':
      case 'F':
        if (sleeveState !== 'failure' && displayCode) {
          e.preventDefault();
          // Flip is handled by Sleeve component via onFlip
        }
        break;
      case ' ':
      case 'Space':
        if (sleeveState === 'unsealed' || sleeveState === 'staged') {
          e.preventDefault();
          // File picker handled by native drag-drop
        }
        break;
    }
  }, [showFirstRun, sleeveState, displayCode, handleSendClick, handleReceiveClick, handleClose, handleRetry, handleDismiss]);

  useEffect(() => {
    window.addEventListener('keydown', handleGlobalKeyDown);
    return () => window.removeEventListener('keydown', handleGlobalKeyDown);
  }, [handleGlobalKeyDown]);

  const handleFilesDropped = useCallback((droppedFiles: File[]) => {
    if (sleeveState === 'unsealed' || sleeveState === 'staged') {
      void beginSendFiles(droppedFiles);
    }
  }, [beginSendFiles, sleeveState]);

  useDragDrop(handleFilesDropped);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    onMenuAction((action) => {
      if (action === 'send') {
        triggerFilePicker();
      } else if (action === 'receive') {
        void handleReceiveClick();
      }
    }).then((fn) => { unlisten = fn; });

    return () => { unlisten?.(); };
  }, [handleReceiveClick, triggerFilePicker]);

  const handleFilePickerChange = useCallback((event: React.ChangeEvent<HTMLInputElement>) => {
    const selectedFiles = Array.from(event.target.files ?? []);
    if (selectedFiles.length > 0) {
      void beginSendFiles(selectedFiles);
    }
    event.target.value = '';
  }, [beginSendFiles]);

  // Live region for screen reader announcements
  useEffect(() => {
    const region = document.createElement('div');
    region.setAttribute('aria-live', 'polite');
    region.setAttribute('aria-atomic', 'true');
    region.style.position = 'absolute';
    region.style.width = '1px';
    region.style.height = '1px';
    region.style.padding = '0';
    region.style.margin = '-1px';
    region.style.overflow = 'hidden';
    region.style.clip = 'rect(0, 0, 0, 0)';
    region.style.whiteSpace = 'nowrap';
    region.style.border = '0';
    document.body.appendChild(region);
    liveRegionRef.current = region;
    return () => { document.body.removeChild(region); liveRegionRef.current = null; };
  }, []);

  const announce = useCallback((message: string, priority: 'polite' | 'assertive' = 'polite') => {
    if (liveRegionRef.current) {
      liveRegionRef.current.setAttribute('aria-live', priority);
      liveRegionRef.current.textContent = message;
      setTimeout(() => { if (liveRegionRef.current) liveRegionRef.current.textContent = ''; }, 1000);
    }
  }, []);

  // Announce state changes
  useEffect(() => {
    const messages: Record<WidgetState, string> = {
      unsealed: 'Ready.',
      staged: `${itemCount} ${itemCount === 1 ? 'item' : 'items'} staged.`,
      sealed: `Sealed. Code ${displayCode}.`,
      transferring: 'Transferring.',
      delivered: 'Delivered.',
      receive: 'Receive mode. Code ' + displayCode + '.',
      failure: `Void. ${failureCause}.`,
    };
    if (messages[sleeveState]) {
      announce(messages[sleeveState], sleeveState === 'failure' ? 'assertive' : 'polite');
    }
  }, [sleeveState, itemCount, displayCode, failureCause, announce]);

  // Announce progress
  useEffect(() => {
    if (bandProgress && sleeveState === 'transferring') {
      const pct = Math.round((bandProgress.current / bandProgress.total) * 100);
      announce(`Transfer ${pct} percent, ${bandProgress.speed ?? ''}`, 'polite');
    }
  }, [bandProgress, sleeveState, announce]);

  const qrCodeSvg = useMemo(() => displayCode ? generateQRCodeSvg(displayCode) : '', [displayCode]);

  const sleeveProps = useMemo(() => ({
    state: sleeveState,
    files,
    displayCode,
    byteTotal,
    progress: bandProgress,
    activeFileId,
    onSeal: handleSendClick,
    onFlip: handleFlip,
    onClose: handleClose,
    onProgress: () => {},
    onDelivered: () => {},
    qrCodeSvg,
    failureCause,
  }), [sleeveState, files, displayCode, byteTotal, bandProgress, activeFileId, handleSendClick, handleFlip, handleClose, qrCodeSvg, failureCause]);

  if (showFirstRun) {
    return (
      <div className="first-run-shell">
        <FirstRunWindow onComplete={handleFirstRunComplete} />
      </div>
    );
  }

  return (
    <MotionConfig reducedMotion={prefersReduced ? "user" : "never"}>
      <div className="app">
        <input
          ref={filePickerRef}
          type="file"
          multiple
          onChange={handleFilePickerChange}
          style={{ display: 'none' }}
          aria-hidden="true"
        />
        <Sleeve {...sleeveProps} />
      </div>
    </MotionConfig>
  );
}

if (typeof window !== 'undefined' && process.env.NODE_ENV === 'development') {
  (window as any).__AIRLYNK_MOCK__ = {
    unsealedState,
    stagedState,
    sealedState,
    transferringState,
    deliveredState,
    receiveState,
    failureStates,
  };
}

export default App;