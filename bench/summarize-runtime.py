#!/usr/bin/env python3
"""Summarize raw interleaved samples with paired bootstrap median intervals."""
import argparse,json,random,statistics
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('directory',type=Path);a=p.parse_args();rng=random.Random(20260930)
def rows(revision,dataset):
 result={}
 files=a.directory.glob(f'{revision}-{dataset}-*.jsonl')
 for f in sorted(files,key=lambda path:int(path.stem.rsplit('-',1)[1])):
  for line in f.read_text().splitlines():
   r=json.loads(line);key=(r.get('mode'),r['workload'],r.get('depth'))
   result.setdefault(key,[]).append(r)
 return result
def median(values):return statistics.median(values)
def interval(ratios):
 boots=sorted(median(rng.choices(ratios,k=len(ratios))) for _ in range(10000))
 return [boots[250],boots[9750]]
summary=[]
for dataset in ['call_metrics','timer_queue_matrix','public']:
 baseline=rows('baseline',dataset);modified=rows('modified',dataset)
 if baseline.keys()!=modified.keys():raise ValueError(f'Mismatched workloads for {dataset}')
 for key,b in baseline.items():
  m=modified[key]
  if len(b)!=len(m):raise ValueError(f'Unpaired samples for {dataset}/{key}: {len(b)} != {len(m)}')
  def us(r):
   return r['p50_ms']*1000 if dataset=='public' else r['elapsed_us']/(r.get('operations') or r['repeats']*r['depth'])
  ratios=[us(y)/us(x) for x,y in zip(b,m)]
  row=dict(dataset=dataset,mode=key[0],workload=key[1],depth=key[2],samples=len(ratios),baseline_us=median([us(r) for r in b]),modified_us=median([us(r) for r in m]),paired_ratio=median(ratios),ratio_ci95=interval(ratios))
  if dataset!='public':
   for label,data in [('baseline',b),('modified',m)]:
    operations=data[0].get('operations') or data[0]['repeats']*data[0]['depth']
    row[label+'_allocations_per_op']=median([r['allocations']/operations for r in data]);row[label+'_bytes_per_op']=median([r['bytes']/operations for r in data])
  summary.append(row)
(a.directory/'comparison-summary.json').write_text(json.dumps(summary,indent=2)+'\n')
for r in summary:
 if r['dataset']=='call_metrics' or r['dataset']=='timer_queue_matrix' and r['workload']=='drain' or r['dataset']=='public' and r['workload']=='tiny':
  print(r['dataset'],r['mode'] or '',r['workload'],r['depth'] or '',round(r['baseline_us'],3),round(r['modified_us'],3),round((r['paired_ratio']-1)*100,1),[round((x-1)*100,1) for x in r['ratio_ci95']])
