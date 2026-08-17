export type WidgetState =
  | 'unsealed'
  | 'staged'
  | 'sealed'
  | 'transferring'
  | 'delivered'
  | 'receive'
  | 'failure';

export type FailureCause =
  | 'firewall'
  | 'isolation'
  | 'phone-left'
  | 'disk-full'
  | 'cancelled';

export type WidgetEvent =
  | { type: 'dragover' }
  | { type: 'drop'; payload: { files: File[] } }
  | { type: 'session-opened'; payload: { displayCode: string } }
  | { type: 'progress'; payload: { progress: number; transferSpeed: number } }
  | { type: 'delivered' }
  | { type: 'receive-click' }
  | { type: 'error'; payload: { cause: FailureCause } }
  | { type: 'reset' }
  | { type: 'qr-close' };

export function nextState(current: WidgetState, event: WidgetEvent): WidgetState {
  switch (current) {
    case 'unsealed':
      if (event.type === 'dragover') return 'staged';
      if (event.type === 'receive-click') return 'receive';
      return current;

    case 'staged':
      if (event.type === 'drop' && event.payload.files.length > 0) return 'sealed';
      if (event.type === 'dragover') return 'staged';
      return 'unsealed';

    case 'sealed':
      if (event.type === 'session-opened') return 'transferring';
      if (event.type === 'error') return 'failure';
      if (event.type === 'reset') return 'unsealed';
      return current;

    case 'transferring':
      if (event.type === 'delivered') return 'delivered';
      if (event.type === 'error') return 'failure';
      if (event.type === 'reset') return 'unsealed';
      return current;

    case 'delivered':
      if (event.type === 'reset') return 'unsealed';
      return current;

    case 'receive':
      if (event.type === 'session-opened') return 'transferring';
      if (event.type === 'qr-close') return 'unsealed';
      if (event.type === 'error') return 'failure';
      return current;

    case 'failure':
      if (event.type === 'reset') return 'unsealed';
      return current;

    default:
      return current;
  }
}

export function getFailureCause(state: WidgetState, event: WidgetEvent): FailureCause | undefined {
  if (state === 'failure' && event.type === 'error') {
    return event.payload.cause;
  }
  return undefined;
}