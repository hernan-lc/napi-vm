import {serve} from '@napi-vm/plugin-sdk';
import {plugin} from './plugin.js';
await serve(plugin, {metadata: {id:'example.greeter', version:'0.1.0'}});
