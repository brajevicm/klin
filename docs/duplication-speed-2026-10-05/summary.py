import json,glob,statistics as st,os
here=os.path.dirname(os.path.abspath(__file__))
for f in sorted(glob.glob(here+'/results/*.jsonl')):
    rows=[json.loads(l) for l in open(f)]
    b=rows[0]
    print(f"## {os.path.basename(f)[:-6]}: files={b['files']} lines={b['lines']} tokens={b['tokens']} cold_extra_ms={b['extra_ms']} index_bytes={b['index_bytes']} region_bytes={b['region_bytes']} function_bytes={b['function_bytes']} functions={b['functions']} indexed_functions={b['indexed_functions']} longest_posting={b['longest_posting']} capped_keys={b['capped_keys']} build_rss_kb={b['rss']//1024}")
    for n in (20,100):
        q=[r for r in rows if r['kind']=='query' and r['changed']==min(n,b['files'])]
        m=lambda k: st.median(r[k] for r in q)
        print(f"  warm{n}: design_c median={m('design_c_ms'):.2f} max={max(r['design_c_ms'] for r in q):.2f} | load={m('load_ms'):.2f} stream={m('stream_ms'):.2f} lookup={m('lookup_ms'):.2f} function={m('function_ms'):.2f} | shared read+parse={m('read_parse_ms'):.2f} | "
              + " ".join(f"{k}={q[0][k]}" for k in ('longest_missed','path_bytes','regions','proven','deferred','function_hits','false_regions','false_functions','true_regions','missed_regions','capped_hits','incomplete'))
              + f" rss_kb={max(r['rss'] for r in q)//1024}")
