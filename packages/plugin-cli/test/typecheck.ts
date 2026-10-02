import {main, build, pack, type ArtifactLock} from '../src/index.mjs';
const result: Promise<unknown> = main(['help']);
const built: Promise<{built: true}> = build('.', 'js');
const packed: Promise<{directory: string; lock: ArtifactLock}> = pack('.', '../package');
void [result, built, packed];
