// Run: node --test book/analytics.test.js
const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const { classify, isConfigured, boot } = require(path.join(__dirname, 'analytics.js'));

const here = (href) => new URL(href);
const site = here('https://docs.mockforge.dev/');

test('referrer hosts map to engines', () => {
  const cases = {
    'https://chatgpt.com/': 'chatgpt',
    'https://chat.openai.com/c/abc': 'chatgpt',
    'https://www.perplexity.ai/search?q=x': 'perplexity',
    'https://claude.ai/': 'claude',
    'https://gemini.google.com/app': 'gemini',
    'https://bard.google.com/': 'gemini',
    'https://copilot.microsoft.com/': 'copilot',
    'https://edgeservices.bing.com/edgesvc/chat?x=1': 'copilot',
    'https://www.meta.ai/': 'meta_ai',
    'https://grok.com/': 'grok',
    'https://x.ai/': 'grok',
    'https://chat.deepseek.com/': 'deepseek',
    'https://chat.mistral.ai/chat': 'mistral',
    'https://you.com/search?q=x': 'you',
    'https://www.phind.com/': 'phind',
  };
  for (const [referrer, engine] of Object.entries(cases)) {
    assert.equal(classify(referrer, site), engine, referrer);
  }
});

test('utm_source alone identifies the engine', () => {
  assert.equal(classify('', here('https://docs.mockforge.dev/?utm_source=chatgpt.com')), 'chatgpt');
  assert.equal(classify('', here('https://docs.mockforge.dev/?utm_source=perplexity')), 'perplexity');
  assert.equal(classify('', here('https://docs.mockforge.dev/?utm_source=claude.ai')), 'claude');
  assert.equal(classify('', here('https://docs.mockforge.dev/?utm_source=Copilot')), 'copilot');
  assert.equal(classify('https://www.google.com/', here('https://docs.mockforge.dev/?utm_source=chatgpt.com')), 'chatgpt');
});

test('non-AI and lookalike sources are not tagged', () => {
  assert.equal(classify('', site), null);
  assert.equal(classify('https://www.google.com/search?q=x', site), null);
  assert.equal(classify('https://www.bing.com/', site), null);
  assert.equal(classify('https://edgeservices.bing.com/other', site), null);
  assert.equal(classify('https://google.com/', site), null);
  assert.equal(classify('https://max.ai/', site), null);
  assert.equal(classify('https://notchatgpt.com/', site), null);
  assert.equal(classify('https://chatgpt.com.evil.example/', site), null);
  assert.equal(classify('not a url', site), null);
  assert.equal(classify('', here('https://docs.mockforge.dev/?utm_source=newsletter')), null);
  assert.equal(classify('', here('https://docs.mockforge.dev/?utm_source=you')), null);
});

test('same-site navigation is never tagged', () => {
  assert.equal(classify('https://docs.mockforge.dev/about.html', site), null);
  assert.equal(classify('https://www.docs.mockforge.dev/', site), null);
  assert.equal(classify('https://docs.mockforge.dev/', here('https://docs.mockforge.dev/x?utm_source=chatgpt.com')), null);
});

test('placeholder key keeps the script inert', () => {
  assert.equal(isConfigured(), false);
  const win = { document: { referrer: 'https://chatgpt.com/' }, location: site };
  boot(win);
  assert.equal(win.posthog, undefined);
});

function runInBrowser(href, referrer, storage) {
  const fs = require('node:fs');
  const vm = require('node:vm');
  const source = fs
    .readFileSync(path.join(__dirname, 'analytics.js'), 'utf8')
    .replace('"__POSTHOG' + '_KEY__"', '"phc_testkey"');
  const scripts = [];
  const gtagCalls = [];
  const win = {
    URL,
    URLSearchParams,
    location: new URL(href),
    sessionStorage: storage,
    gtag: (...args) => gtagCalls.push(args),
    document: {
      referrer,
      createElement: () => ({}),
      getElementsByTagName: () => [{ parentNode: { insertBefore: (el) => scripts.push(el) } }],
    },
  };
  win.window = win;
  vm.runInNewContext(source, win);
  const calls = win.posthog.filter((entry) => Array.isArray(entry));
  return { win, scripts, gtagCalls, calls };
}

function memoryStorage() {
  const data = new Map();
  return { getItem: (k) => (data.has(k) ? data.get(k) : null), setItem: (k, v) => data.set(k, String(v)) };
}

test('configured key tags an AI landing once per session', () => {
  const storage = memoryStorage();
  const first = runInBrowser('https://docs.mockforge.dev/pricing?utm_source=chatgpt.com', 'https://chatgpt.com/', storage);
  assert.equal(first.scripts[0].src, 'https://us-assets.i.posthog.com/static/array.js');
  const [key, config] = first.win.posthog._i[0];
  assert.equal(key, 'phc_testkey');
  assert.equal(config.persistence, 'memory');
  assert.equal(config.disable_session_recording, true);
  const register = first.calls.find((c) => c[0] === 'register');
  assert.deepEqual({ ...register[1] }, { ai_referrer: 'chatgpt' });
  const capture = first.calls.find((c) => c[0] === 'capture');
  assert.equal(capture[1], 'ai_referral_landing');
  assert.equal(capture[2].ai_engine, 'chatgpt');
  assert.equal(capture[2].landing_path, '/pricing');
  assert.equal(capture[2].referrer_host, 'chatgpt.com');
  assert.equal(capture[2].utm_source, 'chatgpt.com');
  assert.equal(capture[2].$set_once.initial_ai_referrer, 'chatgpt');
  assert.equal(first.gtagCalls.length, 1);
  assert.equal(first.gtagCalls[0][1], 'ai_referral_landing');

  const second = runInBrowser('https://docs.mockforge.dev/about', 'https://docs.mockforge.dev/pricing', storage);
  assert.equal(second.calls.some((c) => c[0] === 'capture'), false);
  assert.equal(second.calls.find((c) => c[0] === 'register')[1].ai_referrer, 'chatgpt');
});

test('blocked sessionStorage still tags the landing page', () => {
  const blocked = { getItem() { throw new Error('blocked'); }, setItem() { throw new Error('blocked'); } };
  const run = runInBrowser('https://docs.mockforge.dev/', 'https://www.perplexity.ai/', blocked);
  assert.equal(run.calls.find((c) => c[0] === 'capture')[2].ai_engine, 'perplexity');
});

test('non-AI visit sends no AI events', () => {
  const run = runInBrowser('https://docs.mockforge.dev/', 'https://www.google.com/', memoryStorage());
  assert.equal(run.calls.some((c) => c[0] === 'capture' || c[0] === 'register'), false);
  assert.equal(run.gtagCalls.length, 0);
});
