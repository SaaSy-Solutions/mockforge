"""Generate WireMock mappings for performance runs from OpenAPI fixtures."""
import json, pathlib, re, sys
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from requests import build_operation
spec_dir, out_dir=map(pathlib.Path,sys.argv[1:3]); out_dir.mkdir(parents=True,exist_ok=True)
for path in spec_dir.glob('*.json'):
    spec=json.loads(path.read_text()); mappings=[]
    for url,item in spec.get('paths',{}).items():
        for method,op in item.items():
            if method.lower() not in {'get','post','put','patch','delete','options','head'}: continue
            responses=op.get('responses',{})
            code=next((int(x) for x in responses if x.isdigit() and 200 <= int(x) < 300),200)
            pattern='^'+''.join('[^/]+' if part.startswith('{') and part.endswith('}')
                                else re.escape(part)
                                for part in re.split(r'(\{[^}]+\})',url))+'$'
            body={'id':1,'name':'benchmark pet'} if path.name=='petstore.json' else {}
            mappings.append({'request':{'method':method.upper(),'urlPathPattern': pattern},'response':{'status':code,'jsonBody':body,'headers':{'Content-Type':'application/json'}}})
    (out_dir/(path.stem+'-wiremock.json')).write_text(json.dumps({'mappings':mappings},indent=2))
