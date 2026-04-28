export interface NodeLabStatusRow {
  label: string;
  value: string;
}

export function buildNodeLabStatusRows(): NodeLabStatusRow[] {
  return [
    { label: 'Authoring API', value: 'Unavailable' },
    { label: 'Local Agent', value: 'Unavailable' },
    { label: 'Runtime Publishing', value: 'Unavailable' },
  ];
}

export function nodeLabUnavailableMessage(): string {
  return 'Node authoring is unavailable in this build';
}
