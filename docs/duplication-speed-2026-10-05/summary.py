import json,glob,statistics as st,os,sys
here=os.path.dirname(os.path.abspath(__file__))
results=sys.argv[1] if len(sys.argv)>1 else here+'/results-regions'
for f in sorted(glob.glob(os.path.join(results,'*.jsonl'))):
    rows=[json.loads(l) for l in open(f)]
    b=rows[0]
    print(f"## {os.path.basename(f)[:-6]}: files={b['files']} lines={b['lines']} tokens={b['tokens']} cold_extra_ms={b['extra_ms']} index_bytes={b['index_bytes']} region_bytes={b['region_bytes']} longest_posting={b['longest_posting']} capped_keys={b['capped_keys']} build_rss_kb={b['rss']//1024}")
    for n in (20,100):
        q=[r for r in rows if r['kind']=='query' and r.get('requested_changed',r['changed'])==n]
        m=lambda k: st.median(r[k] for r in q)
        print(f"  warm{n}: actual_changed={q[0]['changed']} tokens={q[0]['changed_tokens']} design_c median={m('design_c_ms'):.2f} max={max(r['design_c_ms'] for r in q):.2f} check median={m('check_ms') if 'check_ms' in q[0] else 0:.2f} | load={m('load_ms'):.2f} stream={m('stream_ms'):.2f} lookup={m('lookup_ms'):.2f} | shared read+parse={m('read_parse_ms'):.2f} | "
              + " ".join(f"{k}={q[0][k]}" for k in ('longest_missed','path_bytes','regions','bridged','unanchored','check_files','check_blocked','check_false','check_missed','longest_check_missed','proven','deferred','false_regions','true_regions','missed_regions','capped_hits','incomplete') if k in q[0])
              + f" rss_kb={max(r['rss'] for r in q)//1024}")
