export type SessionKind = 'send' | 'receive';

export type FileStatus = 'pending' | 'active' | 'done' | 'skipped' | 'failed';

export interface SessionFile {
  id: number;
  originalName: string;
  size: number;
  sentBytes: number;
  status: FileStatus;
  progress?: number;
  error?: string;
}

export interface Session {
  id: number;
  token: string;
  displayCode: string;
  kind: SessionKind;
  key: Uint8Array;
  baseNonce: Uint8Array;
  files: SessionFile[];
  uploadCount: number;
  uploadBytes: number;
  cancelled: boolean;
  lastActivity: number;
  created: number;
}

export type SleeveState =
  | 'unsealed'
  | 'staged'
  | 'sealed'
  | 'transferring'
  | 'delivered'
  | 'receive'
  | 'failure';

export interface SealBandProps {
  status: SleeveState;
  displayCode?: string;
  itemCount: number;
  byteTotal: number;
  progress?: {
    current: number;
    total: number;
    speed?: string;
  };
  onClose: () => void;
  onFlip: () => void;
  onSeal?: () => void;
}

export interface SleeveProps {
  state: SleeveState;
  files: SessionFile[];
  displayCode?: string;
  byteTotal: number;
  progress?: {
    current: number;
    total: number;
    speed?: string;
  };
  activeFileId?: number;
  onSeal: () => void;
  onFlip: () => void;
  onProgress: (progress: { current: number; total: number; speed?: string }) => void;
  onDelivered: () => void;
  onClose: () => void;
  qrCodeSvg?: string;
  failureCause?: string;
}

export interface ContentsProps {
  files: SessionFile[];
  activeFileId?: number;
}

export interface RowProps {
  file: SessionFile;
  isActive: boolean;
}

export interface LabelFaceProps {
  qrCodeSvg: string;
  displayCode: string;
  fileName?: string;
  progress?: { current: number; total: number; speed?: string };
  onClose: () => void;
}

export interface VoidWashProps {
  cause: string;
  onRetry: () => void;
  onDismiss: () => void;
}

export type ReachDiagnosis = 'bound' | 'bindFailed' | 'blockedByFirewall' | 'noPhoneYet';

export type FirewallStatus = 'clean' | 'missingAllowRule' | 'staleBlockRules' | 'unknown';

export interface ReachReport {
  diagnosis: ReachDiagnosis;
  bindAddr: string;
  interfaceName: string;
  detail: string;
}

export interface FirewallReport {
  status: FirewallStatus;
  blocks?: string[];
}

export interface NetworkInfo {
  lanIp: string;
  interfaceName: string;
  port: number;
}