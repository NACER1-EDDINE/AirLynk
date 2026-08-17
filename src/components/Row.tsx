import { memo, useMemo } from 'react';
import { motion } from 'framer-motion';
import { RowProps, SessionFile } from '../types';

const formatBytes = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} GB`;
};

const truncateFilename = (name: string, maxLength = 32): string => {
  if (name.length <= maxLength) return name;
  const extIndex = name.lastIndexOf('.');
  if (extIndex === -1 || extIndex < maxLength - 3) {
    return name.slice(0, maxLength - 1) + '…';
  }
  const namePart = name.slice(0, extIndex);
  const ext = name.slice(extIndex);
  const available = maxLength - ext.length - 1;
  if (available < 5) return name.slice(0, maxLength - 1) + '…';
  return namePart.slice(0, available) + '…' + ext;
};

interface FileGlyphProps {
  status: SessionFile['status'];
  progress?: number;
}

const FileGlyph = memo(function FileGlyph({ status, progress = 0 }: FileGlyphProps) {
  const size = 16;
  const strokeWidth = 1.5;

  const pendingStyle = {
    width: size,
    height: size,
    border: `${strokeWidth}px dashed var(--color-structure)`,
    borderRadius: 'var(--radius-tight)',
    background: 'transparent',
    flexShrink: 0,
  };

  const activeStyle = {
    width: size,
    height: size,
    position: 'relative' as const,
    border: `${strokeWidth}px solid var(--color-structure)`,
    borderRadius: 'var(--radius-tight)',
    background: 'transparent',
    flexShrink: 0,
  };

  const doneStyle = {
    width: size,
    height: size,
    border: `${strokeWidth}px solid var(--color-signal)`,
    borderRadius: 'var(--radius-tight)',
    background: 'transparent',
    flexShrink: 0,
  };

  const skippedStyle = {
    width: size,
    height: size,
    position: 'relative' as const,
    border: `${strokeWidth}px solid var(--color-structure)`,
    borderRadius: 'var(--radius-tight)',
    background: 'transparent',
    flexShrink: 0,
  };

  const failedStyle = {
    width: size,
    height: size,
    borderRadius: '50%',
    background: 'transparent',
    flexShrink: 0,
    position: 'relative' as const,
  };

  switch (status) {
    case 'pending':
      return <div className="file-glyph pending" style={pendingStyle} aria-label="Pending" />;
    case 'active':
      return (
        <div className="file-glyph active" style={activeStyle} aria-label={`${Math.round(progress)}% complete`}>
          <motion.div
            className="glyph-fill"
            animate={{ scaleX: progress / 100 }}
            style={{
              position: 'absolute',
              left: 0,
              top: 0,
              bottom: 0,
              width: '100%',
              background: 'var(--color-signal)',
              borderRadius: `${size}px 0 0 ${size}px`,
              transformOrigin: 'left center',
            }}
          />
        </div>
      );
    case 'done':
      return (
        <div className="file-glyph done" style={doneStyle} aria-label="Complete">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" style={{ display: 'block' }}>
            <polyline
              points="4,8 7,11 12,4"
              stroke="var(--color-signal)"
              strokeWidth={2}
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </div>
      );
    case 'skipped':
      return (
        <div className="file-glyph skipped" style={skippedStyle} aria-label="Skipped">
          <div
            style={{
              position: 'absolute',
              left: '50%',
              top: '50%',
              transform: 'translate(-50%, -50%)',
              width: 8,
              height: strokeWidth,
              background: 'var(--color-structure)',
              borderRadius: 1,
            }}
          />
        </div>
      );
    case 'failed':
      return (
        <div className="file-glyph failed" style={failedStyle} aria-label="Failed">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" style={{ display: 'block' }}>
            <circle cx="8" cy="8" r="7" stroke="var(--color-void)" strokeWidth={strokeWidth} />
            <line x1="5" y1="5" x2="11" y2="11" stroke="var(--color-void)" strokeWidth={strokeWidth} strokeLinecap="round" />
            <line x1="11" y1="5" x2="5" y2="11" stroke="var(--color-void)" strokeWidth={strokeWidth} strokeLinecap="round" />
          </svg>
        </div>
      );
    default:
      return <div className="file-glyph pending" style={pendingStyle} aria-label="Pending" />;
  }
});

export const Row = memo(function Row({ file, isActive }: RowProps) {
  const displayName = useMemo(() => truncateFilename(file.originalName), [file.originalName]);
  const displaySize = useMemo(() => formatBytes(file.size), [file.size]);
  const isErrored = file.status === 'failed';
  const showProgress = isActive && file.status === 'active' && file.progress !== undefined;

  const baseStyle = {
    display: 'flex',
    alignItems: 'center',
    gap: 'var(--space-2)',
    padding: '8px var(--space-1)',
    minHeight: 40,
    fontFamily: 'var(--font-sans)',
    fontSize: 'var(--font-size-micro)',
    color: 'var(--color-ink)',
    borderBottom: '1px solid var(--color-structure)',
    position: 'relative' as const,
  };

  const activeStyle = {
    ...baseStyle,
    background: 'var(--color-window-bg)',
    borderLeft: '2px solid var(--color-signal)',
  };

  const erroredStyle = {
    ...baseStyle,
    background: 'var(--color-band-bg-void)',
    paddingBottom: showProgress ? 8 : 20,
  };

  const rowStyle = isErrored ? erroredStyle : isActive ? activeStyle : baseStyle;

  const progressPercent = file.progress ?? 0;

  return (
    <motion.div
      className="file-row"
      style={rowStyle}
      initial={{ opacity: 0, y: -20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.2, ease: [0.25, 0.46, 0.45, 0.94] }}
    >
      <FileGlyph status={file.status} progress={progressPercent} />
      <div
        className="file-info"
        style={{
          flex: 1,
          minWidth: 0,
          display: 'flex',
          flexDirection: 'column',
          gap: 'var(--space-1)',
        }}
      >
        <span
          className="file-name"
          style={{
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            fontSize: 'var(--font-size-micro)',
            color: 'var(--color-ink)',
          }}
          title={file.originalName}
        >
          {displayName}
        </span>
        <span
          className="file-size"
          style={{
            fontSize: 'var(--font-size-micro)',
            color: 'var(--color-structure)',
            fontVariantNumeric: 'tabular-nums lining-nums',
            fontFamily: 'var(--font-sans)',
          }}
        >
          {displaySize}
        </span>
      </div>
      {showProgress && (
        <div
          className="row-progress"
          style={{
            position: 'absolute',
            bottom: 0,
            left: 0,
            right: 0,
            height: 3,
            background: 'var(--color-structure)',
            overflow: 'hidden',
          }}
        >
          <motion.div
            className="progress-fill"
            animate={{ scaleX: progressPercent / 100 }}
            style={{
              height: '100%',
              background: 'var(--color-signal)',
              transformOrigin: 'left center',
              borderRadius: '0 0 var(--radius-tight) var(--radius-tight)',
            }}
            transition={{ duration: 0.15, ease: 'easeOut' }}
          />
        </div>
      )}
      {isErrored && file.error && (
        <div
          className="file-error"
          style={{
            marginTop: 'var(--space-2)',
            paddingLeft: 26,
            fontSize: 'var(--font-size-micro)',
            color: 'var(--color-void)',
            fontFamily: 'var(--font-sans)',
          }}
        >
          {file.error}
        </div>
      )}
    </motion.div>
  );
});

export { FileGlyph };