export function main(args?: string[]): Promise<unknown>;
export function build(directory: string, format: 'js' | 'executable', options?: {stage?: string}): Promise<{built: true; format: string; directory: string; output: string; target: unknown}>;
export interface ArtifactTarget {os: string; arch: string; libc?: string}
export interface ArtifactLock {
  lockVersion: 1; pluginId: string; pluginVersion: string;
  artifact: {profile: string; target: ArtifactTarget; runtime: string[]; abi: string; availability: {built: boolean; distributed: boolean; tested: string[]}};
  interfaces: Record<string, {version: string; digest: string}>;
  files: Record<string, string>;
}
export function pack(directory: string, out: string, options?: {target?: ArtifactTarget; abi?: string; runtime?: string[]; profile?: string}): Promise<{directory: string; lock: ArtifactLock}>;
