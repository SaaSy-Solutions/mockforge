"""Probe every documented operation and validate status/body against OpenAPI responses."""
import json, pathlib, sys, urllib.error, urllib.request
try: import jsonschema
except ImportError: jsonschema=None
spec=json.load(open(sys.argv[1])); base=sys.argv[2].rstrip('/'); passed=total=0
for path,item in spec.get('paths',{}).items():
    for method,op in item.items():
        if method.lower() not in {'get','post','put','patch','delete','options','head'}: continue
        url=path.replace('{petId}','1').replace('{id}','1')
        total+=1; req=urllib.request.Request(base+url,method=method.upper())
        try:
            with urllib.request.urlopen(req,timeout=3) as r: status=r.status; body=r.read()
        except urllib.error.HTTPError as e: status=e.code; body=e.read()
        declared=op.get('responses',{}).get(str(status)); valid=declared is not None
        if valid and declared.get('content') and jsonschema:
            schema=next(iter(declared['content'].values())).get('schema')
            try: jsonschema.validate(json.loads(body),schema)
            except Exception: valid=False
        if valid: passed+=1
print(json.dumps({'passed':passed,'total':total,'fraction':passed/total if total else None,'validator':'jsonschema' if jsonschema else 'status-only-jsonschema-unavailable'}))
