import { serve } from '@napi-vm/plugin-sdk';
import { plugin } from './plugin.js';
import { PLUGIN_METADATA } from './generated/plugin-metadata.js';
await serve(plugin, { metadata: PLUGIN_METADATA });
