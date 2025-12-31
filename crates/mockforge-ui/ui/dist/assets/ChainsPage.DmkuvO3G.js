import{R as v,j as e,r as i}from"./react-vendor.CjFKJWOU.js";import{a as y,ay as b,l as Q,f as d,G as X,H as he,J as ue,K as A,L,N as q,O as R,U as P,V as F}from"./index.js";import{B as D}from"./Badge.HqBhlCiq.js";import{C as Y,d as z,a as Z,b as ee,c as pe}from"./Card.DLDyS9TF.js";import{L as je,T as fe}from"./textarea.BzH1jVce.js";import{L as N}from"./loader-circle.DayZ6CKk.js";import{P as U}from"./play.DFhY6Mwp.js";import"./chart-vendor.Bws7AqQX.js";import"./query-vendor.CB8uHlA8.js";import"./ui-vendor.B5WCADYB.js";import"./state-vendor.RQj9JU5O.js";const se=v.forwardRef(({className:l,...a},n)=>e.jsx("div",{className:"relative w-full overflow-auto",children:e.jsx("table",{ref:n,className:y("w-full caption-bottom text-sm",l),...a})}));se.displayName="Table";const ae=v.forwardRef(({className:l,...a},n)=>e.jsx("thead",{ref:n,className:y("[&_tr]:border-b",l),...a}));ae.displayName="TableHeader";const te=v.forwardRef(({className:l,...a},n)=>e.jsx("tbody",{ref:n,className:y("[&_tr:last-child]:border-0",l),...a}));te.displayName="TableBody";const I=v.forwardRef(({className:l,...a},n)=>e.jsx("tr",{ref:n,className:y("border-b transition-colors hover:bg-muted/50 data-[state=selected]:bg-muted",l),...a}));I.displayName="TableRow";const p=v.forwardRef(({className:l,...a},n)=>e.jsx("th",{ref:n,className:y("h-12 px-4 text-left align-middle font-medium text-muted-foreground [&:has([role=checkbox])]:pr-0",l),...a}));p.displayName="TableHead";const j=v.forwardRef(({className:l,...a},n)=>e.jsx("td",{ref:n,className:y("p-4 align-middle [&:has([role=checkbox])]:pr-0",l),...a}));j.displayName="TableCell";const Ae=({className:l})=>{const[a,n]=i.useState([]),[E,f]=i.useState(!0),[C,g]=i.useState(null),[m,S]=i.useState(null),[M,c]=i.useState(!1),[x,w]=i.useState(null),[B,O]=i.useState(!1),[ne,T]=i.useState(!1),[ie,V]=i.useState(!1),[o,J]=i.useState(null),[re,k]=i.useState(null),[_,G]=i.useState(!1);i.useEffect(()=>{le()},[]);const le=async()=>{try{f(!0);const s=await b.listChains();n(s.chains),g(null)}catch(s){Q.error("Failed to fetch chains",s);const t=s instanceof Error?s.message.includes("not valid JSON")||s.message.includes("DOCTYPE")?"Chain API is not available. The backend may not be running with chain support enabled.":s.message:"Failed to load chains";g(t)}finally{f(!1)}},de=async()=>{if(x)try{S(x.id),await b.deleteChain(x.id),n(a.filter(s=>s.id!==x.id)),c(!1),w(null)}catch(s){g(s instanceof Error?s.message:"Failed to delete chain")}finally{S(null)}},$=()=>{O(!0)},[r,W]=i.useState(null),[ce,K]=i.useState(!1),oe=async s=>{J(s),T(!0),K(!0),W(null);try{const t=await b.getChain(s.id);W(t)}catch(t){Q.error("Failed to fetch chain details",t)}finally{K(!1)}},H=async s=>{J(s),V(!0),G(!0),k(null);try{const t=await b.executeChain(s.id);k(JSON.stringify(t,null,2))}catch(t){k(`Error: ${t instanceof Error?t.message:"Failed to execute chain"}`)}finally{G(!1)}},me=s=>{w(s),c(!0)};return E?e.jsx("div",{className:`p-6 ${l}`,children:e.jsxs("div",{className:"flex items-center justify-center h-64",children:[e.jsx(N,{className:"h-8 w-8 animate-spin text-muted-foreground"}),e.jsx("span",{className:"ml-2 text-lg",children:"Loading chains..."})]})}):e.jsxs("div",{className:`p-6 ${l}`,children:[e.jsxs("div",{className:"flex justify-between items-center mb-6",children:[e.jsxs("div",{children:[e.jsx("h1",{className:"text-2xl font-bold",children:"Request Chains"}),e.jsx("p",{className:"text-muted-foreground",children:"Manage and execute request chains for complex API workflows"})]}),e.jsxs(d,{onClick:$,children:[e.jsx(X,{className:"h-4 w-4 mr-2"}),"Create Chain"]})]}),C&&e.jsx("div",{className:"mb-6 p-4 bg-destructive/10 border border-destructive/20 rounded-md",children:e.jsx("p",{className:"text-destructive",children:C})}),e.jsx("div",{className:"grid gap-4",children:a.length===0?e.jsx(Y,{children:e.jsx(z,{className:"flex flex-col items-center justify-center h-64",children:e.jsxs("div",{className:"text-center",children:[e.jsx("h3",{className:"text-lg font-medium mb-2",children:"No Chains Found"}),e.jsx("p",{className:"text-muted-foreground mb-4",children:"Create your first request chain to get started with complex API workflow testing."}),e.jsxs(d,{variant:"outline",onClick:$,children:[e.jsx(X,{className:"h-4 w-4 mr-2"}),"Create First Chain"]})]})})}):e.jsxs(Y,{children:[e.jsxs(Z,{children:[e.jsxs(ee,{children:["Available Chains (",a.length,")"]}),e.jsx(pe,{children:"Click on a chain to view details and execute it"})]}),e.jsx(z,{children:e.jsxs(se,{children:[e.jsx(ae,{children:e.jsxs(I,{children:[e.jsx(p,{children:"Name"}),e.jsx(p,{children:"Description"}),e.jsx(p,{children:"Links"}),e.jsx(p,{children:"Status"}),e.jsx(p,{children:"Tags"}),e.jsx(p,{className:"w-48",children:"Actions"})]})}),e.jsx(te,{children:a.map(s=>{var t,h;return e.jsxs(I,{children:[e.jsx(j,{className:"font-medium",children:s.name}),e.jsx(j,{className:"max-w-md truncate",children:s.description||"No description"}),e.jsx(j,{children:s.linkCount}),e.jsx(j,{children:e.jsx(D,{variant:s.enabled?"default":"secondary",children:s.enabled?"Enabled":"Disabled"})}),e.jsx(j,{children:e.jsxs("div",{className:"flex gap-1",children:[(t=s.tags)==null?void 0:t.map(u=>e.jsx(D,{variant:"outline",className:"text-xs",children:u},u)),!((h=s.tags)!=null&&h.length)&&e.jsx("span",{className:"text-muted-foreground",children:"—"})]})}),e.jsx(j,{children:e.jsxs("div",{className:"flex gap-2",children:[e.jsxs(d,{variant:"outline",size:"sm",onClick:()=>oe(s),children:[e.jsx(he,{className:"h-4 w-4 mr-1"}),"View"]}),e.jsxs(d,{variant:"outline",size:"sm",onClick:()=>H(s),disabled:!s.enabled,children:[e.jsx(U,{className:"h-4 w-4 mr-1"}),"Execute"]}),e.jsxs(d,{variant:"outline",size:"sm",onClick:()=>me(s),disabled:m===s.id,children:[m===s.id?e.jsx(N,{className:"h-4 w-4 mr-1 animate-spin"}):e.jsx(ue,{className:"h-4 w-4 mr-1"}),"Delete"]})]})})]},s.id)})})]})})]})}),e.jsx(A,{open:M,onOpenChange:c,children:e.jsxs(L,{children:[e.jsxs(q,{children:[e.jsx(R,{children:"Delete Chain"}),e.jsxs(P,{children:['Are you sure you want to delete the chain "',x==null?void 0:x.name,'"? This action cannot be undone.']})]}),e.jsxs(F,{children:[e.jsx(d,{variant:"outline",onClick:()=>c(!1),disabled:m!==null,children:"Cancel"}),e.jsx(d,{variant:"destructive",onClick:de,disabled:m!==null,children:m!==null?e.jsxs(e.Fragment,{children:[e.jsx(N,{className:"h-4 w-4 mr-2 animate-spin"}),"Deleting..."]}):"Delete"})]})]})}),e.jsx(A,{open:B,onOpenChange:O,children:e.jsxs(L,{className:"max-w-4xl max-h-[90vh] overflow-y-auto",children:[e.jsxs(q,{children:[e.jsx(R,{children:"Create Chain"}),e.jsx(P,{children:"Create a new request chain using YAML definition."})]}),e.jsx(ge,{onClose:()=>O(!1),onSuccess:s=>{n([...a,s]),O(!1)}})]})}),e.jsx(A,{open:ne,onOpenChange:T,children:e.jsxs(L,{className:"max-w-5xl max-h-[90vh] overflow-y-auto",children:[e.jsxs(q,{children:[e.jsx(R,{children:o==null?void 0:o.name}),e.jsx(P,{children:(o==null?void 0:o.description)||"No description provided"})]}),e.jsx("div",{className:"py-4",children:ce?e.jsxs("div",{className:"flex items-center justify-center h-32",children:[e.jsx(N,{className:"h-8 w-8 animate-spin text-muted-foreground"}),e.jsx("span",{className:"ml-2",children:"Loading chain details..."})]}):r?e.jsxs("div",{className:"space-y-6",children:[e.jsxs("div",{children:[e.jsx("h4",{className:"font-medium mb-3",children:"Overview"}),e.jsxs("div",{className:"grid grid-cols-3 gap-4 text-sm",children:[e.jsxs("div",{className:"space-y-1",children:[e.jsx("span",{className:"text-muted-foreground",children:"Status"}),e.jsx("div",{children:e.jsx(D,{variant:r.config.enabled?"default":"secondary",children:r.config.enabled?"Enabled":"Disabled"})})]}),e.jsxs("div",{className:"space-y-1",children:[e.jsx("span",{className:"text-muted-foreground",children:"Links"}),e.jsx("div",{className:"font-medium",children:r.links.length})]}),e.jsxs("div",{className:"space-y-1",children:[e.jsx("span",{className:"text-muted-foreground",children:"Max Length"}),e.jsx("div",{className:"font-medium",children:r.config.maxChainLength})]})]}),r.tags&&r.tags.length>0&&e.jsxs("div",{className:"mt-3",children:[e.jsx("span",{className:"text-sm text-muted-foreground",children:"Tags: "}),r.tags.map(s=>e.jsx(D,{variant:"outline",className:"ml-1 text-xs",children:s},s))]})]}),e.jsxs("div",{children:[e.jsx("h4",{className:"font-medium mb-3",children:"Configuration"}),e.jsx("div",{className:"bg-muted/50 rounded-lg p-4 space-y-2 text-sm",children:e.jsxs("div",{className:"grid grid-cols-2 gap-4",children:[e.jsxs("div",{children:[e.jsx("span",{className:"text-muted-foreground",children:"Global Timeout:"})," ",e.jsxs("span",{className:"font-medium",children:[r.config.globalTimeoutSecs,"s"]})]}),e.jsxs("div",{children:[e.jsx("span",{className:"text-muted-foreground",children:"Parallel Execution:"})," ",e.jsx("span",{className:"font-medium",children:r.config.enableParallelExecution?"Enabled":"Disabled"})]})]})})]}),r.variables&&Object.keys(r.variables).length>0&&e.jsxs("div",{children:[e.jsx("h4",{className:"font-medium mb-3",children:"Variables"}),e.jsx("div",{className:"bg-muted/50 rounded-lg p-4",children:e.jsx("div",{className:"space-y-2 text-sm font-mono",children:Object.entries(r.variables).map(([s,t])=>e.jsxs("div",{className:"flex",children:[e.jsxs("span",{className:"text-blue-600 dark:text-blue-400",children:[s,":"]}),e.jsx("span",{className:"ml-2 text-muted-foreground",children:typeof t=="string"?t:JSON.stringify(t)})]},s))})})]}),e.jsxs("div",{children:[e.jsxs("h4",{className:"font-medium mb-3",children:["Request Links (",r.links.length,")"]}),e.jsx("div",{className:"space-y-4",children:r.links.map((s,t)=>e.jsxs(Y,{className:"overflow-hidden",children:[e.jsx(Z,{className:"pb-3",children:e.jsxs("div",{className:"flex items-center justify-between",children:[e.jsxs("div",{className:"flex items-center gap-2",children:[e.jsx("span",{className:"flex items-center justify-center w-6 h-6 rounded-full bg-primary/10 text-primary text-xs font-medium",children:t+1}),e.jsx(ee,{className:"text-base",children:s.request.id})]}),e.jsx(D,{variant:"outline",className:"font-mono text-xs",children:s.request.method})]})}),e.jsxs(z,{className:"space-y-3 text-sm",children:[e.jsxs("div",{children:[e.jsx("span",{className:"text-muted-foreground",children:"URL:"}),e.jsx("div",{className:"font-mono text-xs bg-muted/50 p-2 rounded mt-1 break-all",children:s.request.url})]}),s.request.headers&&Object.keys(s.request.headers).length>0&&e.jsxs("div",{children:[e.jsx("span",{className:"text-muted-foreground",children:"Headers:"}),e.jsx("div",{className:"font-mono text-xs bg-muted/50 p-2 rounded mt-1 space-y-1",children:Object.entries(s.request.headers).map(([h,u])=>e.jsxs("div",{children:[e.jsxs("span",{className:"text-blue-600 dark:text-blue-400",children:[h,":"]})," ",e.jsx("span",{className:"text-muted-foreground",children:u})]},h))})]}),s.request.body!=null&&e.jsxs("div",{children:[e.jsx("span",{className:"text-muted-foreground",children:"Body:"}),e.jsx("pre",{className:"font-mono text-xs bg-muted/50 p-2 rounded mt-1 overflow-x-auto",children:JSON.stringify(s.request.body,null,2)})]}),s.extract&&Object.keys(s.extract).length>0&&e.jsxs("div",{children:[e.jsx("span",{className:"text-muted-foreground",children:"Extract Variables:"}),e.jsx("div",{className:"font-mono text-xs bg-muted/50 p-2 rounded mt-1 space-y-1",children:Object.entries(s.extract).map(([h,u])=>{const xe=typeof u=="string"?u:String(u);return e.jsxs("div",{children:[e.jsx("span",{className:"text-green-600 dark:text-green-400",children:h})," ←"," ",e.jsx("span",{className:"text-muted-foreground",children:xe})]},h)})})]}),e.jsxs("div",{className:"flex gap-4 text-xs",children:[s.storeAs&&e.jsxs("div",{children:[e.jsx("span",{className:"text-muted-foreground",children:"Store As:"})," ",e.jsx("span",{className:"font-medium",children:s.storeAs})]}),s.dependsOn&&s.dependsOn.length>0&&e.jsxs("div",{children:[e.jsx("span",{className:"text-muted-foreground",children:"Depends On:"})," ",e.jsx("span",{className:"font-medium",children:s.dependsOn.join(", ")})]})]})]})]},s.request.id))})]})]}):e.jsx("div",{className:"text-center text-muted-foreground py-8",children:"Failed to load chain details"})}),e.jsxs(F,{children:[e.jsx(d,{variant:"outline",onClick:()=>T(!1),children:"Close"}),e.jsxs(d,{onClick:()=>{T(!1),o&&H(o)},disabled:!r||!r.config.enabled,children:[e.jsx(U,{className:"h-4 w-4 mr-2"}),"Execute Chain"]})]})]})}),e.jsx(A,{open:ie,onOpenChange:V,children:e.jsxs(L,{className:"max-w-3xl",children:[e.jsxs(q,{children:[e.jsxs(R,{children:["Execute Chain: ",o==null?void 0:o.name]}),e.jsx(P,{children:_?"Executing chain...":"Chain execution results"})]}),e.jsx("div",{className:"py-4",children:_?e.jsxs("div",{className:"flex items-center justify-center h-32",children:[e.jsx(N,{className:"h-8 w-8 animate-spin text-muted-foreground"}),e.jsx("span",{className:"ml-2",children:"Executing chain..."})]}):e.jsx("div",{className:"space-y-4",children:e.jsxs("div",{children:[e.jsx("h4",{className:"font-medium mb-2",children:"Execution Result"}),e.jsx("pre",{className:"bg-muted p-4 rounded-md text-xs overflow-auto max-h-96",children:re||"No result available"})]})})}),e.jsxs(F,{children:[e.jsx(d,{variant:"outline",onClick:()=>{V(!1),k(null)},disabled:_,children:"Close"}),!_&&e.jsxs(d,{onClick:()=>o&&H(o),children:[e.jsx(U,{className:"h-4 w-4 mr-2"}),"Execute Again"]})]})]})})]})},ge=({onClose:l,onSuccess:a})=>{const[n,E]=i.useState(Ne()),[f,C]=i.useState(!1),[g,m]=i.useState(null),S=async()=>{try{C(!0),m(null);const c=await b.createChain(n),w=(await b.listChains()).chains.find(B=>B.id===c.id);a(w||{id:c.id,name:c.id,description:"",tags:[],enabled:!0,linkCount:0})}catch(c){m(c instanceof Error?c.message:"Failed to create chain")}finally{C(!1)}},M=()=>{E(be())};return e.jsxs("div",{className:"space-y-4",children:[g&&e.jsx("div",{className:"p-3 bg-destructive/10 border border-destructive/20 rounded-md",children:e.jsx("p",{className:"text-sm text-destructive",children:g})}),e.jsxs("div",{className:"space-y-2",children:[e.jsxs("div",{className:"flex justify-between items-center",children:[e.jsx(je,{htmlFor:"yaml-definition",children:"YAML Definition"}),e.jsx(d,{variant:"outline",size:"sm",onClick:M,children:"Load Example"})]}),e.jsx(fe,{id:"yaml-definition",value:n,onChange:c=>E(c.target.value),placeholder:"Enter YAML chain definition...",className:"font-mono text-sm min-h-[400px]"}),e.jsx("p",{className:"text-xs text-muted-foreground",children:"Define your chain using YAML format. Include id, name, description, config, links, variables, and tags."})]}),e.jsxs(F,{children:[e.jsx(d,{variant:"outline",onClick:l,disabled:f,children:"Cancel"}),e.jsx(d,{onClick:S,disabled:f,children:f?e.jsxs(e.Fragment,{children:[e.jsx(N,{className:"h-4 w-4 mr-2 animate-spin"}),"Creating..."]}):"Create Chain"})]})]})};function Ne(){return`# Chain Definition
id: my-chain
name: My Request Chain
description: A simple request chain

config:
  enabled: true
  maxChainLength: 10
  globalTimeoutSecs: 60
  enableParallelExecution: false

links:
  - request:
      id: step1
      method: GET
      url: https://api.example.com/data
      headers:
        Content-Type: application/json
    storeAs: step1_response
    dependsOn: []

variables:
  base_url: https://api.example.com

tags:
  - example
`}function be(){return`# Example: User Management Workflow
id: user-workflow-chain
name: User Management Workflow
description: |
  A complete user management workflow that demonstrates request chaining:
  1. Login to get authentication token
  2. Create a new user profile
  3. Update user settings
  4. Verify the user was created

config:
  enabled: true
  maxChainLength: 10
  globalTimeoutSecs: 60
  enableParallelExecution: false

links:
  # Step 1: Authentication - Login to get access token
  - request:
      id: login
      method: POST
      url: https://api.example.com/auth/login
      headers:
        Content-Type: application/json
      body:
        email: "user@example.com"
        password: "secure-password"
    extract:
      token: body.access_token
    storeAs: auth_response
    dependsOn: []

  # Step 2: Create user profile
  - request:
      id: create_user
      method: POST
      url: https://api.example.com/users
      headers:
        Content-Type: application/json
        Authorization: "Bearer {{chain.auth_response.body.access_token}}"
      body:
        name: "John Doe"
        email: "{{chain.auth_response.body.email}}"
        department: "Engineering"
    extract:
      user_id: body.id
      user_name: body.name
    storeAs: user_create_response
    dependsOn:
      - login

  # Step 3: Update user preferences
  - request:
      id: update_preferences
      method: PUT
      url: https://api.example.com/users/{{chain.user_create_response.body.id}}/preferences
      headers:
        Content-Type: application/json
        Authorization: "Bearer {{chain.auth_response.body.access_token}}"
      body:
        theme: dark
        notifications: true
        language: en
    storeAs: preferences_update
    dependsOn:
      - create_user

  # Step 4: Verify user creation
  - request:
      id: verify_user
      method: GET
      url: https://api.example.com/users/{{chain.user_create_response.body.id}}
      headers:
        Authorization: "Bearer {{chain.auth_response.body.access_token}}"
    storeAs: user_verification
    expectedStatus: [200]
    dependsOn:
      - create_user

variables:
  base_url: https://api.example.com
  api_version: v1

tags:
  - authentication
  - user-management
  - workflow
`}export{Ae as ChainsPage};
