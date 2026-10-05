import json,glob,statistics as st,os
here=os.path.dirname(os.path.abspath(__file__))
for f in sorted(glob.glob(here+'/results/*.jsonl')):
    rows=[json.loads(l) for l in open(f)]
    b=rows[0]
    print(f"## {os.path.basename(f)[:-6]}: files={b['files']} lines={b['lines']} tokens={b['tokens']} error_files={b['error_files']} "
          f"cold_total_ms={b['total_ms']} cold_parse_ms={b['read_parse_ms']} cold_extra_ms={b['extra_ms']} index_bytes={b['index_bytes']} "
          f"chain_bytes={b['chain_bytes']} longest_posting={b['longest_posting']} capped_keys={b['capped_keys']} build_rss_kb={b['rss']//1024}")
    for n in (20,100):
        q=[r for r in rows if r['kind']=='query' and r['changed']==min(n,b['files'])]
        m=lambda k: st.median(r[k] for r in q)
        print(f"  warm{n}: design_b median={m('design_b_ms'):.2f} max={max(r['design_b_ms'] for r in q):.2f} | design_a median={m('design_a_ms'):.2f} max={max(r['design_a_ms'] for r in q):.2f} | "
              f"load={m('load_ms'):.2f} stream={m('stream_ms'):.2f} lookup={m('lookup_ms'):.2f} exact={m('exact_ms'):.2f} | shared read+parse={m('read_parse_ms'):.2f} | "
              f"regions={q[0]['regions']} block_b={q[0]['approx_block']} unclear_b={q[0]['approx_unclear']} block_a={q[0]['exact_block']} capped_hits={q[0]['capped_hits']} incomplete={q[0]['incomplete']} "
              f"chain_files={q[0]['chain_files']} chain_bytes={q[0]['chain_bytes']} rss_kb={max(r['rss'] for r in q)//1024}")
