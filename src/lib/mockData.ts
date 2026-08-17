import type { SessionFile } from '../types';

const MB = 1024 * 1024;
const GB = 1024 * MB;

const baseFiles: SessionFile[] = [
  { id: 1, originalName: 'IMG_4471.HEIC', size: 4.2 * MB, sentBytes: 0, status: 'pending' },
  { id: 2, originalName: 'holiday-edit.mp4', size: 1.4 * GB, sentBytes: 0, status: 'pending' },
  { id: 3, originalName: 'receipt-2024.pdf', size: 180 * 1024, sentBytes: 0, status: 'pending' },
  { id: 4, originalName: 'backup.zip', size: 820 * MB, sentBytes: 0, status: 'pending' },
  { id: 5, originalName: 'presentation.pptx', size: 12 * MB, sentBytes: 0, status: 'pending' },
  { id: 6, originalName: 'voice-memo.m4a', size: 3.4 * MB, sentBytes: 0, status: 'pending' },
  { id: 7, originalName: 'screenshot.png', size: 2.1 * MB, sentBytes: 0, status: 'pending' }
];

export interface MockState {
  status: string;
  displayCode: string;
  itemCount: number;
  byteTotal: number;
  files: SessionFile[];
  progress: number;
  transferSpeed: number;
  failureCause?: 'firewall' | 'isolation' | 'phone-left' | 'disk-full' | 'cancelled';
}

export const unsealedState: MockState = {
  status: 'unsealed',
  displayCode: '',
  itemCount: 0,
  byteTotal: 0,
  files: [],
  progress: 0,
  transferSpeed: 0
};

export const stagedState: MockState = {
  status: 'staged',
  displayCode: '',
  itemCount: 3,
  byteTotal: baseFiles[0].size + baseFiles[1].size + baseFiles[2].size,
  files: [
    { ...baseFiles[0], status: 'pending' },
    { ...baseFiles[1], status: 'pending' },
    { ...baseFiles[2], status: 'pending' }
  ],
  progress: 0,
  transferSpeed: 0
};

export const sealedState: MockState = {
  status: 'sealed',
  displayCode: 'K7-R2',
  itemCount: 7,
  byteTotal: baseFiles.reduce((sum, f) => sum + f.size, 0),
  files: baseFiles.map(f => ({ ...f, status: 'pending' })),
  progress: 0,
  transferSpeed: 0
};

export const transferringState: MockState = {
  status: 'transferring',
  displayCode: 'K7-R2',
  itemCount: 7,
  byteTotal: baseFiles.reduce((sum, f) => sum + f.size, 0),
  files: [
    { ...baseFiles[0], sentBytes: baseFiles[0].size, status: 'done' },
    { ...baseFiles[1], sentBytes: Math.floor(baseFiles[1].size * 0.64), status: 'active' },
    { ...baseFiles[2], sentBytes: 0, status: 'pending' },
    { ...baseFiles[3], sentBytes: 0, status: 'pending' },
    { ...baseFiles[4], sentBytes: 0, status: 'pending' },
    { ...baseFiles[5], sentBytes: 0, status: 'pending' },
    { ...baseFiles[6], sentBytes: 0, status: 'pending' }
  ],
  progress: 0.28,
  transferSpeed: 12 * MB
};

export const deliveredState: MockState = {
  status: 'delivered',
  displayCode: 'K7-R2',
  itemCount: 7,
  byteTotal: baseFiles.reduce((sum, f) => sum + f.size, 0),
  files: baseFiles.map(f => ({ ...f, sentBytes: f.size, status: 'done' })),
  progress: 1,
  transferSpeed: 0
};

export const receiveState: MockState = {
  status: 'receive',
  displayCode: 'M4-P9',
  itemCount: 0,
  byteTotal: 0,
  files: [],
  progress: 0,
  transferSpeed: 0
};

export const failureStates: Record<'firewall' | 'isolation' | 'phone-left' | 'disk-full' | 'cancelled', MockState> = {
  firewall: {
    status: 'failure',
    displayCode: 'K7-R2',
    itemCount: 7,
    byteTotal: baseFiles.reduce((sum, f) => sum + f.size, 0),
    files: baseFiles.map((f, i) => ({
      ...f,
      sentBytes: i < 2 ? f.size : 0,
      status: i < 2 ? 'done' : 'failed'
    })),
    progress: 0.28,
    transferSpeed: 0,
    failureCause: 'firewall'
  },
  isolation: {
    status: 'failure',
    displayCode: 'K7-R2',
    itemCount: 7,
    byteTotal: baseFiles.reduce((sum, f) => sum + f.size, 0),
    files: baseFiles.map((f, i) => ({
      ...f,
      sentBytes: i < 1 ? f.size : 0,
      status: i < 1 ? 'done' : 'failed'
    })),
    progress: 0.12,
    transferSpeed: 0,
    failureCause: 'isolation'
  },
  'phone-left': {
    status: 'failure',
    displayCode: 'K7-R2',
    itemCount: 7,
    byteTotal: baseFiles.reduce((sum, f) => sum + f.size, 0),
    files: baseFiles.map((f, i) => ({
      ...f,
      sentBytes: i < 3 ? f.size : 0,
      status: i < 3 ? 'done' : 'failed'
    })),
    progress: 0.45,
    transferSpeed: 0,
    failureCause: 'phone-left'
  },
  'disk-full': {
    status: 'failure',
    displayCode: 'K7-R2',
    itemCount: 7,
    byteTotal: baseFiles.reduce((sum, f) => sum + f.size, 0),
    files: baseFiles.map((f, i) => ({
      ...f,
      sentBytes: i < 4 ? f.size : 0,
      status: i < 4 ? 'done' : 'failed'
    })),
    progress: 0.62,
    transferSpeed: 0,
    failureCause: 'disk-full'
  },
  cancelled: {
    status: 'failure',
    displayCode: 'K7-R2',
    itemCount: 7,
    byteTotal: baseFiles.reduce((sum, f) => sum + f.size, 0),
    files: baseFiles.map((f, i) => ({
      ...f,
      sentBytes: i < 2 ? f.size : 0,
      status: i < 2 ? 'done' : 'failed'
    })),
    progress: 0.28,
    transferSpeed: 0,
    failureCause: 'cancelled'
  }
};