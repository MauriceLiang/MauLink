export type UpdateChannel = 'stable' | 'preview';
export type ReleaseTransition = 'upgrade' | 'downgrade' | 'sameVersion';

export interface ParsedReleaseVersion {
  major: string;
  minor: string;
  patch: string;
  rc: string | null;
}

export function parseReleaseVersion(value: string): ParsedReleaseVersion | null;
export function channelForReleaseVersion(value: string): UpdateChannel;
export function compareReleaseVersions(left: string, right: string): number;
export function classifyReleaseTransition(current: string, candidate: string): ReleaseTransition;
