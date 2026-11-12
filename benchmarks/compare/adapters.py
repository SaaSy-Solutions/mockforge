"""Generate WireMock mappings for performance runs from OpenAPI fixtures."""
import json, pathlib, sys
spec_dir, out_dir=map(pathlib.Path,sys.argv[1:3]); out_dir.mkdir(parents=True,exist_ok=True)
for path in spec_dir.glob('*.json'):
    spec=json.loads(path.read_text()); mappings=[]
    for url,item in spec.get('paths',{}).items():
        for method,op in item.items():
            if method.lower() not in {'get','post','put','patch','delete','options','head'}: continue
            responses=op.get('responses',{}); code=next((int(x) for x in responses if x.isdigit()),200)
            body={'id':1,'name':'benchmark pet'} if path.name=='petstore.json' else {}
            mappings.append({'request':{'method':method.upper(),'urlPathPattern': '^'+url.replace('{','').replace('}','[^/]+')+'$'},'response':{'status':code,'jsonBody':body,'headers':{'Content-Type':'application/json'}}})
    (out_dir/(path.stem+'-wiremock.json')).write_text(json.dumps({'mappings':mappings},indent=2))
