"""Run the pinned servers and collect raw startup, load, RSS, and fidelity data."""
import argparse, datetime, json, os, pathlib, platform, re, shutil, socket, subprocess, tempfile, threading, time, urllib.request
BASE=pathlib.Path(__file__).resolve().parent
def machine():
    cpu='unknown'; ram=None
    try:
        for line in open('/proc/cpuinfo'):
            if line.lower().startswith('model name'): cpu=line.split(':',1)[1].strip(); break
        for line in open('/proc/meminfo'):
            if line.startswith('MemTotal:'): ram=int(line.split()[1])*1024; break
    except OSError: pass
    return {'cpu_model':cpu,'cores':os.cpu_count(),'ram_bytes':ram,'os':platform.platform(),'kernel':platform.release()}
def ready(url, timeout=45):
    until=time.monotonic()+timeout
    while time.monotonic()<until:
        try:
            with urllib.request.urlopen(url,timeout=.3) as r:
                if r.status==200:return
        except Exception: time.sleep(.05)
    raise RuntimeError('server did not return HTTP 200: '+url)
def rss_sampler(pid, stop, output):
    peak=0
    while not stop.is_set():
        try: peak=max(peak,int(open(f'/proc/{pid}/status').read().split('VmRSS:')[1].split()[0])*1024)
        except (OSError,IndexError,ValueError): pass
        stop.wait(.1)
    output.append(peak)
def main():
    p=argparse.ArgumentParser(); p.add_argument('--binary',required=True); p.add_argument('--runs',type=int,default=5); p.add_argument('--concurrency',type=int,default=16); p.add_argument('--duration',default='15s'); p.add_argument('--port',type=int,default=4010); p.add_argument('--load-client',nargs='+',required=True); p.add_argument('--wiremock-version',default='3.13.0'); p.add_argument('--prism-version',default='5.12.0'); p.add_argument('--mockoon-version',default='9.2.0'); a=p.parse_args()
    duration=float(a.duration.rstrip('s')); spec=BASE/'specs/petstore.json'; host=f'http://127.0.0.1:{a.port}'
    with tempfile.TemporaryDirectory(prefix='mockforge-compare-') as temp:
        tmp=pathlib.Path(temp); adapters=tmp/'adapters'; subprocess.run(['python3',str(BASE/'adapters.py'),str(BASE/'specs'),str(adapters)],check=True)
        wire=[('WireMock',a.wiremock_version,['docker','run','--rm','-p',f'{a.port}:8080','-v',f'{adapters}:/home/wiremock/mappings:ro',f'wiremock/wiremock:{a.wiremock_version}'])]
        jar=os.environ.get('WIREMOCK_JAR')
        if jar: wire=[('WireMock',a.wiremock_version,['java','-jar',jar,'--port','8080','--root-dir',str(adapters)])]
        tools=[('MockForge','local-binary',[a.binary,'serve','--spec',str(spec),'--http-port',str(a.port)]),*wire,('Prism',a.prism_version,['npx','--yes',f'@stoplight/prism-cli@{a.prism_version}','mock','-h','0.0.0.0','-p',str(a.port),str(spec)]),('Mockoon',a.mockoon_version,['npx','--yes',f'@mockoon/cli@{a.mockoon_version}','start','--data',str(adapters/'petstore-mockoon.json'),'--port',str(a.port)])]
        all_results=[]
        for name,version,cmd in tools:
            starts=[]
            for _ in range(a.runs):
                started=time.perf_counter(); proc=subprocess.Popen(cmd,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL); ready(host+'/pets'); starts.append(time.perf_counter()-started); proc.terminate(); proc.wait(timeout=15)
            proc=subprocess.Popen(cmd,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL); ready(host+'/pets'); stop=threading.Event(); peaks=[]; sampler=threading.Thread(target=rss_sampler,args=(proc.pid,stop,peaks)); sampler.start()
            c=a.load_client
            if c[0]=='oha': load=[*c,'-c',str(a.concurrency),'-z',a.duration,host+'/pets']
            elif c[0]=='wrk': load=[*c,'-t',str(a.concurrency),'-c',str(a.concurrency),'-d',a.duration,host+'/pets']
            else: load=[*c,host+'/pets','--concurrency',str(a.concurrency),'--duration',str(duration)]
            measured=subprocess.run(load,text=True,capture_output=True,check=True); stop.set(); sampler.join(); proc.terminate(); proc.wait(timeout=15)
            fidelity=subprocess.run(['python3',str(BASE/'fidelity.py'),str(spec),host],text=True,capture_output=True,check=True)
            all_results.append({'tool':name,'version':version,'command':cmd,'metrics':{'cold_start_seconds_median':sorted(starts)[len(starts)//2],'cold_start_run_count':len(starts),'load_client':load,'load_output':measured.stdout.strip(),'fidelity':json.loads(fidelity.stdout),'peak_rss_bytes':max(peaks,default=None)}})
        result={'schema_version':1,'date':datetime.date.today().isoformat(),'host':socket.gethostname(),'machine':machine(),'parameters':{'runs':a.runs,'concurrency':a.concurrency,'duration':a.duration,'warmup_runs_discarded':0},'tools':all_results}
        out=BASE/'results'/f"{result['date']}-{result['host']}.json"; out.parent.mkdir(exist_ok=True); out.write_text(json.dumps(result,indent=2)+'\n'); print(out)
if __name__=='__main__':main()
