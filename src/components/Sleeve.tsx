import { useEffect, useRef, useCallback, useMemo } from 'react';
import { motion, useMotionValue } from 'framer-motion';
import { SleeveProps } from '../types';
import { SealBand } from './SealBand';
import { Contents } from './Contents';
import { LabelFace } from './LabelFace';
import { VoidWash } from './VoidWash';
import {
  sleeveVariants,
  contentVariants,
  flipVariants,
  standardTransitions,
  reducedTransitions,
  prefersReducedMotion,
} from '../lib/motion';

export const Sleeve = ({
  state,
  files,
  displayCode,
  byteTotal,
  progress,
  activeFileId,
  onSeal,
  onFlip,
  onClose,
  qrCodeSvg,
  failureCause,
}: SleeveProps) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const sleeveRef = useRef<HTMLDivElement>(null);
  const isFlippedRef = useRef(false);
  const animationFrameRef = useRef<number | undefined>(undefined);
  const targetRotationRef = useRef({ x: 0, y: 0 });
  const currentRotationRef = useRef({ x: 0, y: 0 });
  const isDraggingOverRef = useRef(false);

  const motionX = useMotionValue(0);
  const motionY = useMotionValue(0);

  const prefersReduced = useMemo(() => prefersReducedMotion(), []);

  const handlePointerMove = useCallback((e: React.PointerEvent) => {
    if (prefersReduced || isDraggingOverRef.current) return;
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;
    const centerX = rect.left + rect.width / 2;
    const centerY = rect.top + rect.height / 2;
    targetRotationRef.current.x = (e.clientY - centerY) / (rect.height / 2);
    targetRotationRef.current.y = (e.clientX - centerX) / (rect.width / 2);
  }, [prefersReduced]);

  const handlePointerLeave = useCallback(() => {
    if (prefersReduced) return;
    targetRotationRef.current = { x: 0, y: 0 };
  }, [prefersReduced]);

  useEffect(() => {
    if (prefersReduced) return;
    const tick = () => {
      currentRotationRef.current.x += (targetRotationRef.current.x - currentRotationRef.current.x) * 0.15;
      currentRotationRef.current.y += (targetRotationRef.current.y - currentRotationRef.current.y) * 0.15;
      motionX.set(currentRotationRef.current.y);
      motionY.set(currentRotationRef.current.x);
      animationFrameRef.current = requestAnimationFrame(tick);
    };
    animationFrameRef.current = requestAnimationFrame(tick);
    return () => {
      if (animationFrameRef.current) cancelAnimationFrame(animationFrameRef.current);
    };
  }, [motionX, motionY, prefersReduced]);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (!isDraggingOverRef.current) {
      isDraggingOverRef.current = true;
      sleeveRef.current?.classList.add('dragover');
    }
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (!e.currentTarget.contains(e.relatedTarget as Node)) {
      isDraggingOverRef.current = false;
      sleeveRef.current?.classList.remove('dragover');
    }
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    isDraggingOverRef.current = false;
    sleeveRef.current?.classList.remove('dragover');
    onSeal();
  }, [onSeal]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      if (isFlippedRef.current) {
        onFlip();
      } else {
        onClose();
      }
    }
  }, [onClose, onFlip]);

  const transitions = useMemo(() => (prefersReduced ? reducedTransitions : standardTransitions), [prefersReduced]);

  return (
    <div
      ref={containerRef}
      className="sleeve-container"
      onPointerMove={handlePointerMove}
      onPointerLeave={handlePointerLeave}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      onKeyDown={handleKeyDown}
      tabIndex={0}
      role="region"
      aria-label="AirLynk transfer sleeve"
      style={{
        width: 'var(--widget-width)',
        height: 'var(--widget-height)',
        perspective: 800,
        transformStyle: 'preserve-3d',
        transformOrigin: 'center center',
        background: 'var(--color-sleeve-bg)',
        borderRadius: 'var(--radius-standard)',
        border: '1px solid var(--color-structure)',
        boxShadow: '0 8px 32px rgba(0,0,0,0.4), inset 0 1px 0 rgba(255,255,255,0.05)',
        overflow: 'hidden',
      }}
    >
      {/* Accessibility scrim for text on translucent surfaces (DESIGN.md §8.1) */}
      <div
        className="scrim"
        style={{
          position: 'absolute',
          inset: 0,
          background: 'var(--color-scrim)',
          pointerEvents: 'none',
          borderRadius: 'var(--radius-standard)',
        }}
        aria-hidden="true"
      />

      <motion.div
        ref={sleeveRef}
        className="sleeve"
        variants={sleeveVariants}
        animate={state}
        transition={transitions.band as any}
        style={{
          width: '100%',
          height: '100%',
          transformStyle: 'preserve-3d',
          transformOrigin: 'center center',
        }}
      >
        <motion.div
          className="layer contents-layer"
          variants={contentVariants}
          animate={state}
          transition={transitions.content as any}
          style={{
            position: 'absolute',
            inset: 0,
            transformStyle: 'preserve-3d',
            transform: `translateZ(0px)`,
            overflow: 'hidden',
            borderRadius: 'var(--radius-standard)',
            paddingTop: 'var(--band-height)',
          }}
        >
          <Contents files={files} activeFileId={activeFileId} />
        </motion.div>

        <motion.div
          className="band-wrapper"
          variants={flipVariants}
          animate={isFlippedRef.current ? 'back' : 'front'}
          transition={transitions.flip as any}
          style={{
            position: 'absolute',
            top: 0,
            left: 0,
            right: 0,
            height: 'var(--band-height)',
            transformStyle: 'preserve-3d',
            transform: `translateZ(4px)`,
            transformOrigin: 'center top',
          }}
        >
          <motion.div
            className="band-outer"
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              right: 0,
              height: 'var(--band-height)',
              transformStyle: 'preserve-3d',
              transformOrigin: 'center top',
              backfaceVisibility: 'hidden',
            }}
          >
            <SealBand
              status={state}
              displayCode={displayCode}
              itemCount={files.length}
              byteTotal={byteTotal}
              progress={progress}
              onClose={onClose}
              onFlip={() => {
                isFlippedRef.current = !isFlippedRef.current;
                onFlip();
              }}
            />
          </motion.div>
          <motion.div
            className="band-inner"
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              right: 0,
              height: 'var(--band-height)',
              transformStyle: 'preserve-3d',
              transform: 'rotateX(180deg) translateZ(-1px)',
              transformOrigin: 'center top',
              backfaceVisibility: 'hidden',
              background: 'var(--color-label-bg)',
              borderBottom: '1px solid var(--color-structure)',
            }}
            aria-hidden="true"
          />
        </motion.div>

        {isFlippedRef.current && qrCodeSvg && (
          <motion.div
            className="label-face"
            variants={flipVariants}
            animate="back"
            transition={transitions.flip as any}
            style={{
              position: 'absolute',
              inset: 0,
              transformStyle: 'preserve-3d',
              transform: 'rotateY(180deg)',
              backfaceVisibility: 'hidden',
            }}
          >
            <LabelFace
              qrCodeSvg={qrCodeSvg}
              displayCode={displayCode ?? ''}
              fileName={files[0]?.originalName ?? 'AirLynk transfer'}
              progress={progress}
              onClose={() => {
                isFlippedRef.current = false;
                onFlip();
              }}
            />
          </motion.div>
        )}

        {state === 'failure' && failureCause && (
          <VoidWash
            cause={failureCause}
            onRetry={onSeal}
            onDismiss={onClose}
          />
        )}
      </motion.div>
    </div>
  );
};