import { serve } from '@napi-vm/plugin-sdk';
import { plugin, CONFIGURATION } from './plugin.mjs';
import { PLUGIN_METADATA } from './generated/plugin-metadata.mjs';
await serve(plugin, { metadata: { ...PLUGIN_METADATA, requiresHost: [CONFIGURATION] } });
