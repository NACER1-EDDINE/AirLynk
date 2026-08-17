import { Variants } from 'framer-motion';

export const prefersReducedMotion = () =>
  typeof window !== 'undefined' &&
  window.matchMedia('(prefers-reduced-motion: reduce)').matches;

export const sleeveVariants: Variants = {
  unsealed: { rotateX: 0, scale: 1, opacity: 1 },
  staged: {
    rotateX: -92,
    scale: 1,
    opacity: 1,
    transition: { duration: 0.3, ease: [0.25, 0.46, 0.45, 0.94] }
  },
  sealed: {
    rotateX: 0,
    scale: 1,
    opacity: 1,
    transition: { duration: 0.4, ease: [0.25, 0.46, 0.45, 0.94] }
  },
  transferring: { rotateX: 0, scale: 1, opacity: 1 },
  delivered: {
    rotateX: 0,
    scale: [1, 1.06, 1],
    rotateZ: [0, -0.6, 0],
    opacity: [1, 0.92, 1],
    transition: { duration: 0.14, ease: [0.25, 0.46, 0.45, 0.94] }
  },
  receive: {
    rotateY: 180,
    rotateX: 0,
    scale: 1,
    opacity: 1,
    transition: { duration: 0.5, ease: [0.25, 0.46, 0.45, 0.94] }
  },
  failure: { rotateX: 0, scale: 1, opacity: 1 }
};

export const bandVariants: Variants = {
  unsealed: { rotateX: 0, opacity: 1 },
  staged: {
    rotateX: -92,
    opacity: 1,
    transition: { duration: 0.3, ease: [0.25, 0.46, 0.45, 0.94] }
  },
  sealed: {
    rotateX: 0,
    opacity: 1,
    transition: { duration: 0.4, ease: [0.25, 0.46, 0.45, 0.94] }
  },
  transferring: { rotateX: 0, opacity: 1 },
  delivered: {
    rotateX: 0,
    opacity: 1,
    scale: [1, 1.06, 1],
    transition: { duration: 0.14, ease: [0.25, 0.46, 0.45, 0.94] }
  },
  receive: {
    rotateY: 180,
    opacity: 0,
    transition: { duration: 0.25, ease: [0.25, 0.46, 0.45, 0.94] }
  },
  failure: { rotateX: 0, opacity: 1 }
};

export const contentVariants: Variants = {
  unsealed: { opacity: 0, translateY: -20 },
  staged: { opacity: 1, translateY: 0, transition: { staggerChildren: 0.04 } },
  sealed: { opacity: 1, translateY: 0 },
  transferring: { opacity: 1, translateY: 0 },
  delivered: { opacity: 1, translateY: 0 },
  receive: { opacity: 1, translateY: 0 },
  failure: { opacity: 1, translateY: 0 },
};

export const rowVariants: Variants = {
  unsealed: { opacity: 0, translateZ: 0, translateY: 20 },
  staged: { opacity: 0, translateZ: 0, translateY: 20 },
  sealed: {
    opacity: 1,
    translateZ: 0,
    translateY: 0,
    transition: { duration: 0.3, ease: [0.25, 0.46, 0.45, 0.94] }
  },
  transferring: { opacity: 1, translateZ: 0, translateY: 0 },
  delivered: { opacity: 1, translateZ: 0, translateY: 0 },
  receive: { opacity: 0, translateZ: -6, translateY: 20, transition: { duration: 0.2, ease: [0.25, 0.46, 0.45, 0.94] } },
  failure: { opacity: 1, translateZ: 0, translateY: 0 }
};

export const filmVariants: Variants = {
  unsealed: { opacity: 0.15 },
  staged: { opacity: 0.15 },
  sealed: { opacity: 0.15 },
  transferring: { opacity: 0.15 },
  delivered: { opacity: 0.15 },
  receive: { opacity: 0 },
  failure: { opacity: 0.15 }
};

export const flipVariants: Variants = {
  front: { rotateY: 0 },
  back: { rotateY: 180 }
};

export const qrVariants: Variants = {
  hidden: { opacity: 0, scale: 0.95 },
  visible: {
    opacity: 1,
    scale: 1,
    transition: { duration: 0.3, ease: [0.25, 0.46, 0.45, 0.94] }
  }
};

export const voidVariants: Variants = {
  hidden: { opacity: 0 },
  visible: {
    opacity: 0.08,
    transition: { duration: 0.3, ease: [0.25, 0.46, 0.45, 0.94] }
  }
};

export const progressVariants: Variants = {
  idle: { width: '0%' },
  active: { width: '100%' },
  complete: { width: '100%' }
};

export const standardTransitions = {
  band: { duration: 0.35, ease: [0.25, 0.46, 0.45, 0.94] },
  content: { duration: 0.25, ease: [0.25, 0.46, 0.45, 0.94] },
  flip: { duration: 0.5, ease: [0.25, 0.46, 0.45, 0.94] },
  stamp: { duration: 0.14, ease: 'easeOut' },
  stagger: 0.04,
};

export const reducedTransitions = {
  band: { duration: 0.12, ease: 'linear' },
  content: { duration: 0.12, ease: 'linear' },
  flip: { duration: 0.12, ease: 'linear' },
  stamp: { duration: 0.12, ease: 'linear' },
};

export const parallaxConfig = {
  maxAngle: 1.5,
  layers: 4
};