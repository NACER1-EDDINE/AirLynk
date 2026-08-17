import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn, Event } from '@tauri-apps/api/event';

const generateFallbackDisplayCode = (): string => {
  const alphabet = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789';
  const pick = () => alphabet[Math.floor(Math.random() * alphabet.length)];
  return `${pick()}${pick()}-${pick()}${pick()}`;
};

export interface StartSendSessionParams {
  files: File[];
}

export interface StartSendSessionResult {
  token: string;
  displayCode: string;
}

export interface StartReceiveSessionResult {
  token: string;
  displayCode: string;
}

export interface EndSessionParams {
  token: string;
}

export interface CancelTransferParams {
  token: string;
}

export interface PickFilesResult {
  files: File[];
}

export interface FirewallStatusResult {
  status: 'clean' | 'missingAllowRule' | 'staleBlockRules' | 'unknown';
  blocks?: string[];
}

export interface NetworkInfoResult {
  lanIp: string;
  interfaceName: string;
  port: number;
}

export interface VerifyReachabilityParams {
  addr: string;
  iface: string;
}

export interface ReachabilityResult {
  diagnosis: 'bound' | 'bindFailed' | 'blockedByFirewall' | 'noPhoneYet';
  bindAddr: string;
  interfaceName: string;
  detail: string;
}

export interface ProgressEvent {
  token: string;
  sentBytes: number;
  totalBytes: number;
  files: Array<{
    id: number;
    sentBytes: number;
    status: 'pending' | 'active' | 'done' | 'failed';
  }>;
  transferSpeed: number;
}

export const startSendSession = async (files: File[]): Promise<StartSendSessionResult> => {
  try {
    return await invoke<StartSendSessionResult>('start_send_session', { files });
  } catch (error) {
    console.warn('Tauri send session command unavailable; using mock session token.', error);
    const displayCode = generateFallbackDisplayCode();
    return { token: displayCode, displayCode };
  }
};

export const startReceiveSession = async (): Promise<StartReceiveSessionResult> => {
  try {
    return await invoke<StartReceiveSessionResult>('start_receive_session');
  } catch (error) {
    console.warn('Tauri receive session command unavailable; using mock session token.', error);
    const displayCode = generateFallbackDisplayCode();
    return { token: displayCode, displayCode };
  }
};

export const endSession = (token: string): Promise<void> =>
  invoke('end_session', { token });

export const cancelTransfer = (token: string): Promise<void> =>
  invoke('cancel_transfer', { token });

export const pickFiles = async (): Promise<PickFilesResult> => {
  try {
    return await invoke<PickFilesResult>('pick_files');
  } catch (error) {
    console.warn('Tauri file picker unavailable; falling back to in-app selection.', error);
    return { files: [] };
  }
};

export const checkFirewall = (): Promise<FirewallStatusResult> =>
  invoke('check_firewall');

export const repairFirewall = (): Promise<void> =>
  invoke('repair_firewall');

export const getNetworkInfo = (): Promise<NetworkInfoResult> =>
  invoke('get_network_info');

export const verifyReachability = (addr: string, iface: string): Promise<ReachabilityResult> =>
  invoke('verify_reachability', { addr, iface });

export const onMenuAction = (
  callback: (action: 'send' | 'receive') => void
): Promise<UnlistenFn> =>
  listen('menu-action', (event: Event<'send' | 'receive'>) => callback(event.payload));

export const onProgress = (callback: (progress: ProgressEvent) => void): Promise<UnlistenFn> =>
  listen('transfer-progress', (event: Event<ProgressEvent>) => callback(event.payload));

export const onSessionEvent = (
  eventName: 'session-opened' | 'session-delivered' | 'session-error' | 'session-cancelled',
  callback: (data: { token: string; displayCode?: string; cause?: string }) => void
): Promise<UnlistenFn> =>
  listen(eventName, (event: Event<{ token: string; displayCode?: string; cause?: string }>) =>
    callback(event.payload)
  );