import React, { useEffect, useState } from 'react'

type ReplayItem = { protocol:string, operation_id:string, saved_at:string, path:string }

function useJson(url:string){
    const [data,setData]=useState<any>(null)
    const [err,setErr]=useState<string>('')
    useEffect(()=>{ fetch(url).then(r=>r.json()).then(setData).catch(e=>setErr(String(e))) },[url])
    return {data,err}
  }

export default function App(){
  const [state, setState] = useState<any>(null)
  const [replays, setReplays] = useState<ReplayItem[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    ;(async () => {
      try{
        const s = await fetch('/__admin/api/state').then(r => r.json())
        setState(s)
        const rs = await fetch('/__admin/api/replay').then(r => r.json())
        setReplays(rs.items || rs)
      }catch(e:any){
        setError(e?.message || 'Failed to load')
      }finally{
        setLoading(false)
      }
    })()
  }, [])

  if(loading) return <div style={{padding:20}}>Loading…</div>
  if(error) return <div style={{padding:20, color:'crimson'}}>Error: {error}</div>

  return (
    <div style={{padding:20, fontFamily:'system-ui'}}>
      <h1>MockForge Admin</h1>
      <section style={{marginTop:16}}>
        <h2>Server State</h2>
        <pre style={{background:'#f6f6f6', padding:12, borderRadius:8, overflowX:'auto'}}>
          {JSON.stringify(state, null, 2)}
        </pre>
      </section>
      <section style={{marginTop:16}}>
        <h2>Record/Replay Fixtures</h2>
        <table style={{width:'100%', borderCollapse:'collapse'}}>
          <thead>
            <tr>
              <th style={{textAlign:'left', borderBottom:'1px solid #ddd'}}>Protocol</th>
              <th style={{textAlign:'left', borderBottom:'1px solid #ddd'}}>Operation</th>
              <th style={{textAlign:'left', borderBottom:'1px solid #ddd'}}>Saved At</th>
              <th style={{textAlign:'left', borderBottom:'1px solid #ddd'}}>Path</th>
            </tr>
          </thead>
          <tbody>
          {replays.map((r, i) => (
            <tr key={i}>
              <td>{r.protocol}</td>
              <td>{r.operation_id}</td>
              <td>{r.saved_at}</td>
              <td><code>{r.path}</code></td>
            </tr>
          ))}
          </tbody>
        </table>
      </section>
    </div>
  )
}
