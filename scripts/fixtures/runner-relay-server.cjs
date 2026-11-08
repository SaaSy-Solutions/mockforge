// Synthetic callback collector and echo backend. No production endpoints or data.
const http = require('node:http');
const nonce = process.env.FIXTURE_NONCE;
const token = process.env.FIXTURE_TOKEN;
const ids = JSON.parse(process.env.FIXTURE_RUN_IDS);
const reservation = process.env.FIXTURE_RESERVATION_ID;
const state = { jobs: Object.fromEntries(ids.map(id => [id, { start: [], events: [], finish: [] }])), targets: [], lookups: [], auth_failures: 0 };
const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, 'http://fixture');
  const send = (status, body) => {
    res.writeHead(status, { 'content-type': 'application/json', 'x-fixture-proof': nonce });
    res.end(JSON.stringify(body));
  };
  let raw = '';
  for await (const chunk of req) {
    raw += chunk;
    if (raw.length > 65536) { send(413, { error: 'fixture body limit' }); return; }
  }
  let body = null;
  if (raw) {
    try { body = JSON.parse(raw); } catch { send(400, { error: 'fixture JSON only' }); return; }
  }
  if (url.pathname === '/health') { send(200, { healthy: true }); return; }
  if (url.pathname === '/state') { send(200, state); return; }
  if (url.pathname.startsWith('/target/')) {
    const observed = { method: req.method, path: url.pathname, query: Object.fromEntries(url.searchParams), proof: req.headers['x-fixture-proof'] || null, body };
    state.targets.push(observed);
    send(req.method === 'POST' ? 201 : 200, { nonce, ...observed });
    return;
  }
  if (url.pathname.startsWith('/api/v1/internal/')) {
    if (req.headers.authorization !== `Bearer ${token}`) {
      state.auth_failures++;
      send(403, { error: 'synthetic token required' }); return;
    }
    const callback = url.pathname.match(/^\/api\/v1\/internal\/test-runs\/([^/]+)\/(start|events|finish)$/);
    if (callback && state.jobs[callback[1]] && req.method === 'POST') {
      const job = state.jobs[callback[1]];
      job[callback[2]].push(body);
      // Real registry start is idempotent: queued -> running, otherwise 200/noop.
      if (callback[2] === 'start') {
        send(200, { status: job.start.length === 1 ? 'running' : 'noop' }); return;
      }
      send(200, { accepted: true }); return;
    }
    const lookup = url.pathname.match(/^\/api\/v1\/internal\/tunnel-reservations\/by-subdomain\/([^/]+)$/);
    if (lookup && req.method === 'GET') {
      state.lookups.push(lookup[1]);
      if (lookup[1] === 'reserved-fixture') {
        send(200, { id: reservation, org_id: '00000000-0000-4000-8000-000000000001', name: 'fixture', subdomain: 'reserved-fixture', status: 'reserved' });
      } else { send(404, { error: 'no synthetic reservation' }); }
      return;
    }
  }
  send(404, { error: 'unknown fixture endpoint' });
});
server.listen(8080, '0.0.0.0');
