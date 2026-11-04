export interface NodeLabStatusRow { label: string; value: string }
export function buildNodeLabStatusRows(): NodeLabStatusRow[] {
  return [
    { label: 'Graph authoring', value: 'Host-admitted nodes' },
    { label: 'Local Agent', value: 'Unavailable' },
    { label: 'Runtime Publishing', value: 'Unsupported' },
  ];
}
export function nodeLabTrustMessage(): string {
  return 'These nodes are supplied by this host and run as trusted in-process code. NodeLab creates workflows; it cannot install or publish code.';
}
