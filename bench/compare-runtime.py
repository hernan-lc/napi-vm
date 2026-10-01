#!/usr/bin/env python3
"""Interleave already-built baseline/modified executables; emit raw JSONL.
Builds must be complete and no competing builds/benchmarks may run.
"""
import argparse, os, subprocess
from pathlib import Path
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--baseline', type=Path, required=True)
parser.add_argument('--modified', type=Path, default=Path(__file__).resolve().parents[1])
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--samples', type=int, default=7)
parser.add_argument('--selected-threshold', type=int, default=128)
parser.add_argument('--threshold-binaries', type=Path,
                    help='optional directory containing napi-timers-16/32/64 executables')
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
for sample in range(args.samples):
    order = ['baseline', 'modified'] if sample % 2 == 0 else ['modified', 'baseline']
    for revision in order:
        directory = (args.baseline if revision == 'baseline' else args.modified).resolve()
        for example in ['call_metrics', 'timer_queue_matrix']:
            with (args.output / f'{revision}-{example}-{sample}.jsonl').open('w') as out:
                subprocess.run([str(directory / 'target/release/examples' / example)], stdout=out, check=True)
        env = os.environ.copy()
        env['RUNTIME_BINDING'] = str(directory / 'index.js')
        with (args.output / f'{revision}-public-{sample}.jsonl').open('w') as out:
            subprocess.run(['node', str(args.modified.resolve() / 'bench/runtime-matrix.cjs')], env=env, stdout=out, check=True)
    if args.threshold_binaries:
        for threshold in ([16, 32, 64, 128] if sample % 2 == 0 else [128, 64, 32, 16]):
            binary = args.modified / 'target/release/examples/timer_queue_matrix' if threshold == args.selected_threshold else args.threshold_binaries / f'napi-timers-{threshold}'
            with (args.output / f'threshold-{threshold}-{sample}.jsonl').open('w') as out:
                subprocess.run([str(binary.resolve())], stdout=out, check=True)
