#!/usr/bin/env python3
"""Small standard-library HTTP load client used when oha and wrk are absent."""
import argparse, concurrent.futures, time, urllib.request

def main():
    p=argparse.ArgumentParser(); p.add_argument('url'); p.add_argument('--concurrency',type=int,default=16); p.add_argument('--duration',type=float,default=15); a=p.parse_args()
    lat=[]; end=time.monotonic()+a.duration
    def request():
        start=time.perf_counter()
        with urllib.request.urlopen(a.url, timeout=5) as r: r.read(); return time.perf_counter()-start
    with concurrent.futures.ThreadPoolExecutor(max_workers=a.concurrency) as pool:
        futures=set()
        while time.monotonic()<end or futures:
            while time.monotonic()<end and len(futures)<a.concurrency: futures.add(pool.submit(request))
            done,futures=concurrent.futures.wait(futures,timeout=.05,return_when=concurrent.futures.FIRST_COMPLETED)
            for f in done:
                try: lat.append(f.result())
                except Exception: pass
    lat.sort()
    def pct(q): return lat[min(len(lat)-1,int((len(lat)-1)*q))] if lat else None
    print({'requests':len(lat),'throughput_rps':len(lat)/a.duration,'p50_ms':pct(.50)*1000 if lat else None,'p95_ms':pct(.95)*1000 if lat else None,'p99_ms':pct(.99)*1000 if lat else None})
if __name__=='__main__': main()
