/*
 * PostHog analytics + AI-referral tagging.
 *
 * Inert until POSTHOG_KEY holds a real project key (PostHog project keys are
 * public by design; they only allow sending events). While the value is the
 * placeholder, this file loads nothing and sends nothing.
 *
 * Privacy: cookieless (persistence: 'memory'), no session recording, and
 * autocapture limited to links, buttons and form submits (never input fields).
 * The only browser storage used is one sessionStorage key that remembers which
 * AI assistant referred this tab, so later pages in the same visit keep the tag.
 */
(function (root, factory) {
  var api = factory();
  if (typeof module === 'object' && module.exports) {
    module.exports = api;
  } else if (root && root.document) {
    api.boot(root);
  }
})(typeof window !== 'undefined' ? window : this, function () {
  'use strict';

  var POSTHOG_KEY = "__POSTHOG_KEY__";
  var POSTHOG_API_HOST = 'https://us.i.posthog.com';
  var POSTHOG_UI_HOST = 'https://us.posthog.com';
  var POSTHOG_SCRIPT = 'https://us-assets.i.posthog.com/static/array.js';
  var SESSION_KEY = 'ai_referrer';

  // engine -> referrer hosts (a host also matches its subdomains).
  var ENGINE_HOSTS = {
    chatgpt: ['chatgpt.com', 'chat.openai.com'],
    perplexity: ['perplexity.ai'],
    claude: ['claude.ai'],
    gemini: ['gemini.google.com', 'bard.google.com'],
    copilot: ['copilot.microsoft.com'],
    meta_ai: ['meta.ai'],
    grok: ['grok.com', 'x.ai'],
    deepseek: ['chat.deepseek.com'],
    mistral: ['chat.mistral.ai'],
    you: ['you.com'],
    phind: ['phind.com']
  };

  // Bare utm_source values (no dot) that name an engine.
  var UTM_ALIASES = {
    chatgpt: 'chatgpt',
    openai: 'chatgpt',
    perplexity: 'perplexity',
    claude: 'claude',
    gemini: 'gemini',
    bard: 'gemini',
    copilot: 'copilot',
    meta_ai: 'meta_ai',
    metaai: 'meta_ai',
    grok: 'grok',
    deepseek: 'deepseek',
    mistral: 'mistral',
    phind: 'phind'
  };

  function hostMatches(host, domain) {
    return host === domain || host.slice(-(domain.length + 1)) === '.' + domain;
  }

  function engineForHost(host, path) {
    if (!host) return null;
    host = host.toLowerCase().replace(/\.$/, '');
    if (host === 'edgeservices.bing.com') {
      return /^\/edgesvc\/chat/i.test(path || '') ? 'copilot' : null;
    }
    for (var engine in ENGINE_HOSTS) {
      var hosts = ENGINE_HOSTS[engine];
      for (var i = 0; i < hosts.length; i++) {
        if (hostMatches(host, hosts[i])) return engine;
      }
    }
    return null;
  }

  function engineForUtm(source) {
    if (!source) return null;
    var value = String(source).trim().toLowerCase();
    if (!value) return null;
    if (Object.prototype.hasOwnProperty.call(UTM_ALIASES, value)) return UTM_ALIASES[value];
    var host = value.replace(/^[a-z]+:\/\//, '').replace(/^www\./, '').split(/[/?#]/)[0];
    var path = value.replace(/^[a-z]+:\/\//, '').slice(host.length);
    return host.indexOf('.') === -1 ? null : engineForHost(host, path);
  }

  function parseUrl(value) {
    if (!value) return null;
    try {
      return new URL(String(value));
    } catch (e) {
      return null;
    }
  }

  function bareHost(host) {
    return String(host || '').toLowerCase().replace(/^www\./, '');
  }

  function utmSource(loc) {
    if (!loc) return null;
    var search = loc.search != null ? loc.search : (parseUrl(loc.href) || {}).search;
    if (!search) return null;
    try {
      return new URLSearchParams(search).get('utm_source');
    } catch (e) {
      return null;
    }
  }

  // classify(document.referrer, location) -> engine name | null
  function classify(referrer, loc) {
    var ref = parseUrl(referrer);
    var hereHost = loc && loc.hostname ? bareHost(loc.hostname) : '';
    if (ref && hereHost && bareHost(ref.hostname) === hereHost) return null;
    return (ref && engineForHost(ref.hostname, ref.pathname)) || engineForUtm(utmSource(loc));
  }

  function isConfigured() {
    return typeof POSTHOG_KEY === 'string' && /^phc_[A-Za-z0-9_-]+$/.test(POSTHOG_KEY);
  }

  function readSession(win) {
    try {
      return win.sessionStorage.getItem(SESSION_KEY);
    } catch (e) {
      return null;
    }
  }

  function writeSession(win, value) {
    try {
      win.sessionStorage.setItem(SESSION_KEY, value);
    } catch (e) {
      /* storage blocked: tag this page only */
    }
  }

  // Official posthog-js loader stub: queues calls until array.js arrives.
  function loadPostHog(win) {
    var doc = win.document;
    var ph = win.posthog = win.posthog || [];
    if (ph.__SV) return ph;
    ph._i = [];
    ph.init = function (key, config, name) {
      function stub(target, method) {
        var parts = method.split('.');
        if (parts.length === 2) {
          target = target[parts[0]];
          method = parts[1];
        }
        target[method] = function () {
          target.push([method].concat(Array.prototype.slice.call(arguments, 0)));
        };
      }
      var script = doc.createElement('script');
      script.type = 'text/javascript';
      script.crossOrigin = 'anonymous';
      script.async = true;
      script.src = POSTHOG_SCRIPT;
      var first = doc.getElementsByTagName('script')[0];
      first.parentNode.insertBefore(script, first);
      var instance = ph;
      if (name !== undefined) instance = ph[name] = [];
      else name = 'posthog';
      instance.people = instance.people || [];
      instance.toString = function (noStub) {
        var label = 'posthog';
        if (name !== 'posthog') label += '.' + name;
        if (!noStub) label += ' (stub)';
        return label;
      };
      instance.people.toString = function () {
        return instance.toString(1) + '.people (stub)';
      };
      var methods = 'init capture register register_once register_for_session unregister unregister_for_session getFeatureFlag getFeatureFlagPayload isFeatureEnabled reloadFeatureFlags onFeatureFlags onSessionId identify setPersonProperties group resetGroups reset get_distinct_id getGroups get_session_id alias set_config opt_in_capturing opt_out_capturing has_opted_in_capturing has_opted_out_capturing clear_opt_in_out_capturing debug'.split(' ');
      for (var i = 0; i < methods.length; i++) stub(instance, methods[i]);
      ph._i.push([key, config, name]);
    };
    ph.__SV = 1;
    return ph;
  }

  function boot(win) {
    if (!isConfigured()) return;
    var posthog = loadPostHog(win);
    posthog.init(POSTHOG_KEY, {
      api_host: POSTHOG_API_HOST,
      ui_host: POSTHOG_UI_HOST,
      persistence: 'memory',
      person_profiles: 'identified_only',
      capture_pageview: true,
      capture_pageleave: true,
      disable_session_recording: true,
      disable_surveys: true,
      autocapture: { element_allowlist: ['a', 'button', 'form'] }
    });

    var loc = win.location;
    var engine = classify(win.document.referrer, loc);
    var tagged = readSession(win);

    if (engine && !tagged) {
      writeSession(win, engine);
      var ref = parseUrl(win.document.referrer);
      posthog.register({ ai_referrer: engine });
      posthog.capture('ai_referral_landing', {
        ai_engine: engine,
        landing_path: loc.pathname,
        referrer_host: ref ? ref.hostname : null,
        utm_source: utmSource(loc),
        $set_once: { initial_ai_referrer: engine }
      });
      if (typeof win.gtag === 'function') {
        win.gtag('event', 'ai_referral_landing', { ai_engine: engine });
      }
    } else if (tagged) {
      // Memory persistence resets per page; carry the tag through the visit.
      posthog.register({ ai_referrer: tagged });
    }
  }

  return { classify: classify, boot: boot, isConfigured: isConfigured };
});
