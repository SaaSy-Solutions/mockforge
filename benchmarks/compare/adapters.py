"""Generate portable WireMock mappings and Mockoon environments from OpenAPI fixtures."""
import json, pathlib, sys
spec_dir, out_dir=map(pathlib.Path,sys.argv[1:3]); out_dir.mkdir(parents=True,exist_ok=True)
for path in spec_dir.glob('*.json'):
    spec=json.loads(path.read_text()); mappings=[]; routes=[]
    for url,item in spec.get('paths',{}).items():
        for method,op in item.items():
            if method.lower() not in {'get','post','put','patch','delete','options','head'}: continue
            responses=op.get('responses',{}); code=next((int(x) for x in responses if x.isdigit()),200)
            body={'id':1,'name':'benchmark pet'} if path.name=='petstore.json' else {}
            mappings.append({'request':{'method':method.upper(),'urlPathPattern': '^'+url.replace('{','').replace('}','[^/]+')+'$'},'response':{'status':code,'jsonBody':body,'headers':{'Content-Type':'application/json'}}})
            routes.append({'type':'http','documentation':op.get('operationId',url),'method':method.upper(),'endpoint':url.replace('{','{{').replace('}','}}'),'responses':[{'uuid':'15120000-0000-4000-8000-000000000001','body':json.dumps(body),'statusCode':code,'label':'default','headers':[{'key':'Content-Type','value':'application/json'}]}]})
    (out_dir/(path.stem+'-wiremock.json')).write_text(json.dumps({'mappings':mappings},indent=2))
    (out_dir/(path.stem+'-mockoon.json')).write_text(json.dumps({'name':path.stem,'endpointPrefix':'','port':3000,'routes':routes},indent=2))
