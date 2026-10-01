import type {Contract} from '@napi-vm/plugin-protocol';
export interface GenerationResult { id: string; digest: string; files: string[] }
export function canonical(value: unknown): string;
export function normalize(file: string): Promise<Contract>;
export function generate(file: string, outDir: string, options?: {check?: boolean}): Promise<GenerationResult>;
export function generateAll(options?: {check?: boolean}): Promise<GenerationResult[]>;
export function generatePluginMetadata(plugin: {id: string; version: string; requiresHost?: Contract[]}, outDir: string, options?: {check?: boolean}): Promise<string[]>;
