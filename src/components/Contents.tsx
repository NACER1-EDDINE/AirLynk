import { memo } from 'react';
import { ContentsProps } from '../types';
import { Row } from './Row';

export const Contents = memo(function Contents({ files, activeFileId }: ContentsProps) {
  if (files.length === 0) {
    return (
      <div
        className="contents-empty"
        style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          height: '100%',
          padding: 'var(--space-5) var(--space-4)',
          color: 'var(--color-ink)',
          fontSize: 'var(--font-size-text)',
          fontFamily: 'var(--font-sans)',
        }}
      >
        <div
          className="empty-icon"
          style={{
            width: 48,
            height: 48,
            border: '1px dashed var(--color-structure)',
            borderRadius: 'var(--radius-tight)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: 'var(--space-3)',
            color: 'var(--color-structure)',
          }}
        >
          <svg width="20" height="20" viewBox="0 0 20 20" fill="none" style={{ opacity: 0.7 }}>
            <rect x="2" y="4" width="16" height="14" rx="2" stroke="currentColor" strokeWidth="1.5" />
            <path d="M6 8h8M6 12h5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
        </div>
        <p style={{ margin: 0, textAlign: 'center' }}>Drag files here to begin</p>
      </div>
    );
  }

  return (
    <div
      className="contents"
      style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        overflow: 'hidden',
        padding: 'var(--space-2) var(--space-3) var(--space-3)',
        boxSizing: 'border-box',
      }}
    >
      <div
        className="contents-list"
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 0,
          overflowY: 'auto',
          maxHeight: '100%',
          paddingRight: 'var(--space-1)',
        }}
      >
        {files.map((file) => (
          <Row key={file.id} file={file} isActive={file.id === activeFileId} />
        ))}
      </div>
    </div>
  );
});