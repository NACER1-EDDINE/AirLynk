import { memo, useCallback } from 'react';
import { SealBandProps, SleeveState } from '../types';

const formatBytes = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} GB`;
};

const formatSpeed = (bytesPerSec: number): string => {
  if (bytesPerSec < 1024) return `${bytesPerSec} B/s`;
  if (bytesPerSec < 1024 * 1024) return `${(bytesPerSec / 1024).toFixed(1)} MB/s`;
  return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} GB/s`;
};

interface BandTextProps {
  status: SleeveState;
  displayCode?: string;
  itemCount: number;
  byteTotal: number;
  progress?: { current: number; total: number; speed?: string };
}

const getBandText = (props: BandTextProps): string => {
  const { status, displayCode, itemCount, byteTotal, progress } = props;
  switch (status) {
    case 'unsealed':
      return 'UNSEALED · READY';
    case 'staged':
      return `UNSEALED · ${itemCount} ${itemCount === 1 ? 'ITEM' : 'ITEMS'} · ${formatBytes(byteTotal)}`;
    case 'sealed':
      return `SEALED · ${displayCode ?? ''}`;
    case 'transferring': {
      const pct = progress ? Math.round((progress.current / progress.total) * 100) : 0;
      const speed = progress?.speed ? formatSpeed(Number(progress.speed)) : '';
      return `SEALED · ${displayCode ?? ''} · ${pct}%${speed ? ` · ${speed}` : ''}`;
    }
    case 'delivered':
      return `DELIVERED · ${displayCode ?? ''}`;
    case 'receive':
      return `RECEIVE · ${displayCode ?? ''}`;
    case 'failure':
      return 'VOID · UNKNOWN ERROR';
  }
};

const isVoidState = (status: SleeveState) => status === 'failure';

export const SealBand = memo(function SealBand({
  status,
  displayCode,
  itemCount,
  byteTotal,
  progress,
  onClose,
  onFlip,
  onSeal,
}: SealBandProps) {
  const bandText = getBandText({
    status,
    displayCode,
    itemCount,
    byteTotal,
    progress,
  });
  const isVoid = isVoidState(status);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' && status === 'staged' && onSeal) {
        onSeal();
      } else if (e.key === 'Escape' && status === 'transferring') {
        onClose();
      } else if (e.key === 'Enter' && status === 'delivered') {
        onClose();
      } else if (e.key === 'Escape' && status === 'receive') {
        onFlip();
      } else if (e.key === 'Enter' && status === 'failure') {
        // Retry - handled by parent
      } else if (e.key === 'Escape' && status === 'failure') {
        onClose();
      }
    },
    [status, onClose, onFlip, onSeal]
  );

  const controlButtonStyle = {
    background: 'rgba(255,255,255,0.02)',
    border: '1px solid rgba(255,255,255,0.08)',
    color: isVoid ? 'var(--color-void)' : 'var(--color-structure)',
    fontFamily: 'var(--font-sans)',
    fontSize: 18,
    lineHeight: 1,
    cursor: 'pointer',
    flexShrink: 0,
    width: 32,
    height: 32,
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    padding: 0,
    borderRadius: 9,
  };

  const controlHandlers = {
    onMouseEnter: (e: React.MouseEvent<HTMLButtonElement>) => {
      e.currentTarget.style.color = isVoid ? 'var(--color-void)' : 'var(--color-ink)';
    },
    onMouseLeave: (e: React.MouseEvent<HTMLButtonElement>) => {
      e.currentTarget.style.color = isVoid ? 'var(--color-void)' : 'var(--color-structure)';
    },
    onFocus: (e: React.FocusEvent<HTMLButtonElement>) => {
      e.currentTarget.style.outline = `2px solid ${isVoid ? 'var(--color-void)' : 'var(--color-signal)'}`;
      e.currentTarget.style.outlineOffset = '2px';
    },
    onBlur: (e: React.FocusEvent<HTMLButtonElement>) => {
      e.currentTarget.style.outline = 'none';
    },
  };

  return (
    <div
      className="seal-band"
      data-tauri-drag-region
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        height: 'var(--band-height)',
        padding: '0 var(--space-4)',
        background: isVoid ? 'var(--color-band-bg-void)' : 'var(--color-band-bg)',
        borderBottom: '1px solid var(--color-structure)',
        fontFamily: 'var(--font-sans)',
        fontSize: 'var(--font-size-display)',
        fontWeight: 600,
        lineHeight: 'var(--line-height-tight)',
        letterSpacing: '0.08em',
        color: isVoid ? 'var(--color-void)' : 'var(--color-ink)',
        cursor: 'pointer',
        userSelect: 'none',
      }}
      onKeyDown={handleKeyDown}
      tabIndex={0}
    >
      <span
        data-tauri-drag-region
        style={{ flex: 1, textAlign: 'center', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}
        role="status"
        aria-live="polite"
        aria-atomic="true"
      >
        {bandText}
      </span>
      <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
        <button
          style={controlButtonStyle}
          {...controlHandlers}
          onClick={onFlip}
          aria-label={status === 'failure' ? 'Dismiss panel' : 'Flip to QR label'}
        >
          ⇄
        </button>
        <button
          style={controlButtonStyle}
          {...controlHandlers}
          onClick={onClose}
          aria-label="Close AirLynk"
        >
          ×
        </button>
      </div>
    </div>
  );
});