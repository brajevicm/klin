#!/usr/bin/env python3
"""Fixed per-language limits for #610. Usage: limits.py SIZES CLONES FULL_CLONES > results/limits.txt

Prints the share of corpus A production files over each candidate limit, and for corpus B
the files that first cross each limit in a change (told once: over the limit at the head,
and new or at or under it at the base)."""
import json, os, subprocess, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
os.chdir(os.path.dirname(os.path.abspath(__file__)))
import windows as W
sizes,clones,full=sys.argv[1],sys.argv[2],sys.argv[3]
LIM={'rust':{'prodcode':[400,500,600],'production':[20,25,30]},'ts':{'code':[250,300,400],'named':[12,15,20]}}
sel=json.load(open('corpus/selection.json'))
for lang,key in [('rust','rust'),('typescript','ts')]:
    rows=[]
    for e in (e for e in sel if e['language']==lang):
        out=subprocess.run([sizes,'scan',f"{clones}/{e['repository'].replace('/','__')}",e['commit']],capture_output=True,text=True).stdout
        rows+= [r for r in W.parse(out,None) if r['lang']==key and W.production(r)]
    for m,ks in LIM[key].items():
        print(lang,m,{k:f"{100*sum(r[m]>k for r in rows)/len(rows):.1f}% over" for k in ks})
# corpus B: told-once crossings
from pathlib import Path
B=json.load(open(W.EVIDENCE/'selection.json'))
for key in ['rust','ts']:
  for m,ks in LIM[key].items():
    for k in ks:
      cross=changes=0
      for repo in B['repositories']:
        if W.key_of(repo['language'])!=key: continue
        clone=f"{full}/{repo['fullName'].replace('/','__')}"
        for ch in repo['changes']:
          d=W.run(sizes,'change',clone,ch['base'],ch['head']).splitlines()
          h=d[0].split('\t'); sides={'base':{},'head':{}}
          for line in d[1:]:
            r=dict(zip(h,line.split('\t')))
            for kk in W.METRICS+['test','generated','error','longest']: r[kk]=int(r[kk])
            if r['lang']==key: sides[r['side']][r['path']]=r
          c=sum(1 for p,r in sides['head'].items() if W.production(r) and r[m]>k and (p not in sides['base'] or sides['base'][p][m]<=k))
          cross+=c; changes+=c>0
      print('B',key,m,k,'crossings',cross,'changes',changes,'of 50')
