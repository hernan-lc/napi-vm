import {generate, normalize, generateAll, generatePluginMetadata, type GenerationResult} from '../src/index.mjs';
import type {Contract} from '@napi-vm/plugin-protocol';
const contract: Promise<Contract> = normalize('contract.interface.json');
const result: Promise<GenerationResult> = generate('contract.interface.json','generated',{check:true});
const results: Promise<GenerationResult[]> = generateAll({check:true});
const metadata: Promise<string[]> = generatePluginMetadata({id:'example.test',version:'1.0.0'},'generated');
void [contract,result,results,metadata];
