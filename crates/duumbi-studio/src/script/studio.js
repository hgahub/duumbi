'use strict';

// ── DUUMBI Studio — merged client JS ──────────────────────────────────────────
// Sources:
//   1. New design shell  (design/duumbi_studio.html <script>)
//   2. Existing SSR JS   (crates/duumbi-studio/src/script/studio.js)
//   3. New StudioWS module (see WebSocket section below)
// ─────────────────────────────────────────────────────────────────────────────

(function () {

  // ── Helpers ──────────────────────────────────────────────────────────────────

  function qs(sel, ctx) { return (ctx || document).querySelector(sel); }
  function qsa(sel, ctx) { return Array.prototype.slice.call((ctx || document).querySelectorAll(sel)); }

  // ── State ────────────────────────────────────────────────────────────────────

  // C4 navigation state
  var currentLevel    = 'context';
  var currentModule   = null;
  var currentFunction = null;
  var currentBlock    = null;

  // Code-view toggle state
  var codeViewActive = false;
  var lastGraphData  = null;

  // Filter state
  var activeFilters      = {};
  var filterPopupVisible = false;

  // Type → colour map (used for filter dots and node styling)
  var TYPE_COLORS = {
    person:          '#58a6ff',
    system:          '#388bfd',
    external:        '#8b949e',
    container:       '#a371f7',
    component:       '#3fb950',
    boundary:        '#30363d',
    module:          '#388bfd',
    'function':      '#a371f7',
    block:           '#3fb950',
    'component-dead':'#6e7681',
    'component-sub': '#d2a8ff'
  };

  // Design-shell sidebar state
  var activeFunction = null;
  var sidebarOpen    = false;
  var explorerOpen   = false;
  var isPinned       = false;
  var sidebarWidth   = 220;

  var fnTitles = {
    intents:  'Intents',
    graph:    'Graph',
    build:    'Build'
  };

  // ── Theme ─────────────────────────────────────────────────────────────────────

  var globalTheme  = 'dark';
  var canvasTheme  = 'auto';  // 'auto' | 'light' | 'dark'

  var themeBtn = qs('.theme-toggle');
  if (themeBtn) {
    themeBtn.addEventListener('click', function () {
      if (globalTheme === 'dark') {
        globalTheme = 'light';
        document.body.classList.remove('theme-dark');
        document.body.classList.add('theme-light');
        themeBtn.textContent = '\u{1F319}';
        themeBtn.title = 'Dark mode';
      } else {
        globalTheme = 'dark';
        document.body.classList.remove('theme-light');
        document.body.classList.add('theme-dark');
        themeBtn.textContent = '\u2600';
        themeBtn.title = 'Light mode';
      }
      applyCanvasTheme();
    });
  }

  function applyCanvasTheme() {
    var gc = qs('.graph-canvas-container');
    if (!gc) return;
    var resolved = canvasTheme === 'auto' ? globalTheme : canvasTheme;
    gc.classList.remove('canvas-light', 'canvas-dark');
    gc.classList.add(resolved === 'light' ? 'canvas-light' : 'canvas-dark');
    updateCanvasThemeBtn();
  }

  function toggleCanvasTheme() {
    var opposite = globalTheme === 'dark' ? 'light' : 'dark';
    canvasTheme   = canvasTheme === 'auto' ? opposite : 'auto';
    applyCanvasTheme();
  }

  function updateCanvasThemeBtn() {
    var btn = qs('.canvas-theme-btn');
    if (!btn) return;
    var resolved = canvasTheme === 'auto' ? globalTheme : canvasTheme;
    if (resolved === 'dark') {
      btn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>';
      btn.title = 'Light canvas';
    } else {
      btn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>';
      btn.title = 'Dark canvas';
    }
  }

  // ── Sidebar layout (design shell) ─────────────────────────────────────────────

  function computeLeft() {
    var railEl = document.getElementById('iconRail');
    var rail   = (railEl && railEl.classList.contains('visible')) ? 44 : 0;
    return sidebarOpen ? rail + sidebarWidth : rail;
  }

  function applyLayout(animate) {
    var sb     = document.getElementById('sidebar');
    var canvas = document.getElementById('canvas');
    if (!sb || !canvas) return;
    var railEl = document.getElementById('iconRail');
    var rail   = (railEl && railEl.classList.contains('visible')) ? 44 : 0;

    if (!animate) {
      sb.style.transition     = 'none';
      canvas.style.transition = 'none';
      requestAnimationFrame(function () {
        sb.style.transition     = '';
        canvas.style.transition = '';
      });
    }

    sb.style.left = rail + 'px';
    if (sidebarOpen) {
      sb.style.width = sidebarWidth + 'px';
      sb.classList.add('open');
    } else {
      sb.style.width = '0px';
      sb.classList.remove('open');
    }
    canvas.style.left = computeLeft() + 'px';
  }

  function openSidebar() {
    sidebarOpen  = true;
    explorerOpen = true;
    var railBtn = qs('.rail-btn');
    if (railBtn) railBtn.classList.add('active');
    var hdrToggle = document.getElementById('headerToggle');
    if (hdrToggle) hdrToggle.classList.add('active');
    syncPinUI();
    applyLayout(true);
  }

  function closeSidebarUI() {
    sidebarOpen  = false;
    explorerOpen = false;
    var hdrToggle = document.getElementById('headerToggle');
    if (hdrToggle) hdrToggle.classList.remove('active');
    var railBtn = qs('.rail-btn');
    if (railBtn) railBtn.classList.remove('active');
    applyLayout(true);
  }

  function closeSidebarFull() {
    isPinned = false;
    closeSidebarUI();
    syncPinUI();
  }

  function toggleExplorer() {
    if (explorerOpen) {
      closeSidebarUI();
    } else {
      if (activeFunction) switchPage(activeFunction);
      openSidebar();
    }
  }

  function toggleSidebarFromHeader() {
    if (sidebarOpen) closeSidebarUI();
    else openSidebar();
  }

  function togglePin() {
    if (isPinned) {
      isPinned = false;
      closeSidebarUI();
    } else {
      isPinned = true;
    }
    syncPinUI();
  }

  function syncPinUI() {
    var p = document.getElementById('pinBtn');
    if (!p) return;
    if (isPinned) p.classList.add('pinned');
    else p.classList.remove('pinned');
    p.title = isPinned ? 'Unpin sidebar' : 'Pin sidebar';
  }

  // ── Footer functions ───────────────────────────────────────────────────────────

  function toggleFunction(name) {
    if (activeFunction === name) { deactivateFunction(); return; }
    activeFunction = name;
    qsa('.footer-item').forEach(function (fi) { fi.classList.remove('active'); });
    var btn = qs('.footer-item[data-fn="' + name + '"]');
    if (btn) btn.classList.add('active');
    var railEl = document.getElementById('iconRail');
    if (railEl) railEl.classList.add('visible');
    var hdrToggle = document.getElementById('headerToggle');
    if (hdrToggle) hdrToggle.classList.add('visible');
    switchPage(name);
    openSidebar();
  }

  function switchPage(name) {
    qsa('.sb-page').forEach(function (p) { p.classList.remove('active'); });
    var page = document.getElementById('page-' + name);
    if (page) page.classList.add('active');
    var title = document.getElementById('sidebarTitle');
    if (title) title.textContent = fnTitles[name] || 'Explorer';
  }

  function deactivateFunction() {
    activeFunction = null;
    isPinned       = false;
    qsa('.footer-item').forEach(function (fi) { fi.classList.remove('active'); });
    closeSidebarUI();
    var railEl = document.getElementById('iconRail');
    if (railEl) railEl.classList.remove('visible');
    var hdrToggle = document.getElementById('headerToggle');
    if (hdrToggle) hdrToggle.classList.remove('visible', 'active');
    var railBtn = qs('.rail-btn');
    if (railBtn) railBtn.classList.remove('active');
    closeWorkspaceView();
  }

  // ── Sidebar resize ─────────────────────────────────────────────────────────────

  (function () {
    var handle   = document.getElementById('sidebarResize');
    var sbInner  = document.getElementById('sidebarInner');
    var sb       = document.getElementById('sidebar');
    var canvas   = document.getElementById('canvas');
    if (!handle || !sbInner || !sb || !canvas) return;

    var dragging = false, startX, startW;

    handle.addEventListener('mousedown', function (e) {
      e.preventDefault();
      e.stopPropagation();
      dragging = true;
      startX   = e.clientX;
      startW   = sidebarWidth;
      handle.classList.add('active');
      document.body.classList.add('resizing');
      sb.style.transition     = 'none';
      canvas.style.transition = 'none';
    });

    document.addEventListener('mousemove', function (e) {
      if (!dragging) return;
      var nw = Math.min(420, Math.max(160, startW + (e.clientX - startX)));
      sidebarWidth      = nw;
      sbInner.style.width = nw + 'px';
      sb.style.width      = nw + 'px';
      canvas.style.left   = computeLeft() + 'px';
    });

    document.addEventListener('mouseup', function () {
      if (!dragging) return;
      dragging = false;
      handle.classList.remove('active');
      document.body.classList.remove('resizing');
      sb.style.transition     = '';
      canvas.style.transition = '';
    });
  }());

  // ── Split (md-panel ↔ chat-panel) resize ─────────────────────────────────────

  (function () {
    var handle = document.getElementById('splitResize');
    var md     = document.getElementById('mdPanel');
    var ch     = document.getElementById('chatPanel');
    if (!handle || !md || !ch) return;

    var dragging = false, startX, startMd, startCh;

    handle.addEventListener('mousedown', function (e) {
      e.preventDefault();
      dragging = true;
      startX   = e.clientX;
      startMd  = md.getBoundingClientRect().width;
      startCh  = ch.getBoundingClientRect().width;
      handle.classList.add('active');
      document.body.classList.add('resizing');
    });

    document.addEventListener('mousemove', function (e) {
      if (!dragging) return;
      var tot = startMd + startCh;
      var nm  = Math.max(200, Math.min(tot - 260, startMd + (e.clientX - startX)));
      md.style.flex  = 'none';
      md.style.width = nm + 'px';
      ch.style.width = (tot - nm) + 'px';
    });

    document.addEventListener('mouseup', function () {
      if (!dragging) return;
      dragging = false;
      handle.classList.remove('active');
      document.body.classList.remove('resizing');
    });
  }());

  // ── Intent tree ───────────────────────────────────────────────────────────────

  function toggleIntent(slug) {
    // Capitalize first letter for element ID: "calculator" → "intentCalculator"
    var capSlug = slug.charAt(0).toUpperCase() + slug.slice(1);
    var el = document.getElementById('intent' + capSlug);
    var ch = document.getElementById('children-' + slug);
    if (!el || !ch) return;
    if (el.classList.contains('expanded')) {
      el.classList.remove('expanded', 'active');
      ch.classList.remove('open');
      closeWorkspaceView();
    } else {
      // Collapse all other intents first
      qsa('.tree-intent.expanded').forEach(function (ti) { ti.classList.remove('expanded', 'active'); });
      qsa('.tree-children.open').forEach(function (tc) { tc.classList.remove('open'); });
      el.classList.add('expanded', 'active');
      ch.classList.add('open');
      // Load intent content into md-panel
      loadIntentContent(slug);
      openWorkspaceView();
    }
  }

  function selectIntent(slug) {
    if (!activeFunction || activeFunction !== 'intents') toggleFunction('intents');
    var capSlug = slug.charAt(0).toUpperCase() + slug.slice(1);
    var el = document.getElementById('intent' + capSlug);
    if (el) el.classList.add('expanded', 'active');
    var ch = document.getElementById('children-' + slug);
    if (ch) ch.classList.add('open');
    loadIntentContent(slug);
    openWorkspaceView();
  }

  function loadIntentContent(slug) {
    var mdContent = document.getElementById('mdContent');
    if (!mdContent) return;
    mdContent.innerHTML = '<p style="color:#908c82">Loading intent...</p>';
    fetch('/api/intent/' + encodeURIComponent(slug))
      .then(function (r) { return r.json(); })
      .then(function (data) {
        if (data.error) {
          mdContent.innerHTML = '<p style="color:#f09090">Error: ' + data.error + '</p>';
        } else {
          mdContent.innerHTML = data.html || '<h1>' + data.intent + '</h1><p>Status: ' + data.status + '</p>';
        }
      })
      .catch(function (err) {
        mdContent.innerHTML = '<p style="color:#f09090">Failed to load: ' + err.message + '</p>';
      });
  }

  function selectC4() { openWorkspaceView(); }

  function openWorkspaceView() {
    var wv = document.getElementById('workspaceView');
    if (wv) wv.classList.add('active');
  }

  function closeWorkspaceView() {
    var wv = document.getElementById('workspaceView');
    if (wv) wv.classList.remove('active');
    var md = document.getElementById('mdPanel');
    var ch = document.getElementById('chatPanel');
    if (md) { md.style.flex = ''; md.style.width = ''; }
    if (ch) ch.style.width = '380px';
  }

  // ── Create-intent popup ───────────────────────────────────────────────────────

  function openCreateIntent(e) {
    if (e) e.stopPropagation();
    var bd       = document.getElementById('cipBackdrop');
    var intentEl = document.getElementById('cipIntent');
    var createEl = document.getElementById('cipCreateBtn');
    if (!bd) return;
    bd.classList.add('open');
    if (intentEl) intentEl.value = '';
    if (createEl) { createEl.disabled = true; createEl.textContent = 'Create'; }
    setTimeout(function () { if (intentEl) intentEl.focus(); }, 60);
  }

  function closeCreateIntent() {
    var bd = document.getElementById('cipBackdrop');
    if (bd) bd.classList.remove('open');
  }

  function validateCip() {
    var intentEl = document.getElementById('cipIntent');
    var createEl = document.getElementById('cipCreateBtn');
    if (createEl) createEl.disabled = !(intentEl && intentEl.value.trim());
  }

  function createNewIntent() {
    var intentEl = document.getElementById('cipIntent');
    if (!intentEl) return;
    var desc = intentEl.value.trim();
    if (!desc) return;

    var btn = document.getElementById('cipCreateBtn');
    if (btn) { btn.disabled = true; btn.textContent = 'Creating\u2026'; }

    fetch('/api/intent/create', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ description: desc })
    })
    .then(function (r) { return r.json(); })
    .then(function (data) {
      if (data.error) { throw new Error(data.error); }
      closeCreateIntent();
      addIntentToTree(data.slug);
      selectIntent(data.slug);
    })
    .catch(function (err) {
      if (btn) { btn.textContent = 'Create'; btn.disabled = false; }
      alert('Intent creation failed: ' + err.message);
    });
  }

  function addIntentToTree(slug) {
    var section = qs('#page-intents .sidebar-section');
    if (!section) return;

    var capSlug = slug.charAt(0).toUpperCase() + slug.slice(1);
    // Don't add if already exists
    if (document.getElementById('intent' + capSlug)) return;

    var intentEl = document.createElement('div');
    intentEl.className = 'tree-intent';
    intentEl.id = 'intent' + capSlug;
    intentEl.setAttribute('onclick', "window.__studio.toggleIntent('" + slug + "')");
    intentEl.innerHTML =
      '<svg class="intent-chevron" viewBox="0 0 10 10">' +
        '<path d="M3 2L7 5L3 8" stroke="currentColor" stroke-width="1.3" fill="none" stroke-linecap="round" stroke-linejoin="round"/>' +
      '</svg>' +
      '<svg class="tree-icon" viewBox="0 0 12 12" style="opacity:.8">' +
        '<circle cx="6" cy="6" r="5" stroke="currentColor"/>' +
        '<circle cx="6" cy="6" r="2" stroke="currentColor"/>' +
      '</svg>' +
      '<span>' + slug + '</span>';

    var childrenEl = document.createElement('div');
    childrenEl.className = 'tree-children';
    childrenEl.id = 'children-' + slug;
    childrenEl.innerHTML =
      '<div class="tree-child" onclick="window.__studio.selectC4(\'context\')"><span class="child-dot" style="background:#6fd8b2"></span>Context<span class="tree-badge tb-fn" style="margin-left:auto">C4</span></div>' +
      '<div class="tree-child" onclick="window.__studio.selectC4(\'container\')"><span class="child-dot" style="background:#9ac4ef"></span>Container<span class="tree-badge tb-mod" style="margin-left:auto">C4</span></div>' +
      '<div class="tree-child" onclick="window.__studio.selectC4(\'component\')"><span class="child-dot" style="background:#e07830"></span>Component<span class="tree-badge" style="margin-left:auto;background:#352618;color:#e07830">C4</span></div>' +
      '<div class="tree-child" onclick="window.__studio.selectC4(\'code\')"><span class="child-dot" style="background:#c25a1a"></span>Code<span class="tree-badge" style="margin-left:auto;background:#351a1a;color:#f09090">C4</span></div>';

    section.appendChild(intentEl);
    section.appendChild(childrenEl);
  }

  function executeIntent(slug) {
    if (!slug) return;
    var mdContent = document.getElementById('mdContent');
    if (mdContent) mdContent.innerHTML = '<p style="color:#908c82">Executing intent...</p>';
    fetch('/api/intent/' + encodeURIComponent(slug) + '/execute', { method: 'POST' })
      .then(function (r) { return r.json(); })
      .then(function (data) {
        if (mdContent) {
          mdContent.innerHTML = '<h1>Intent Execution</h1><p>' + (data.message || '') + '</p><pre>' + ((data.log || []).join('\n')) + '</pre>';
        }
        reloadCurrentGraph([]);
      })
      .catch(function (err) {
        if (mdContent) mdContent.innerHTML = '<p style="color:#f09090">Execute failed: ' + err.message + '</p>';
      });
  }

  function runBuild() {
    var mdContent = document.getElementById('mdContent');
    if (mdContent) mdContent.innerHTML = '<p style="color:#908c82">Building...</p>';
    fetch('/api/build', { method: 'POST' })
      .then(function (r) { return r.json(); })
      .then(function (data) {
        if (mdContent) {
          mdContent.innerHTML = '<h1>Build</h1><p>' + (data.message || '') + '</p><p><code>' + (data.output_path || '') + '</code></p>';
        }
      })
      .catch(function (err) {
        if (mdContent) mdContent.innerHTML = '<p style="color:#f09090">Build failed: ' + err.message + '</p>';
      });
  }

  function runBinary() {
    var mdContent = document.getElementById('mdContent');
    if (mdContent) mdContent.innerHTML = '<p style="color:#908c82">Running...</p>';
    fetch('/api/run', { method: 'POST' })
      .then(function (r) { return r.json(); })
      .then(function (data) {
        if (mdContent) {
          mdContent.innerHTML = '<h1>Run</h1><p>Exit code: ' + data.exit_code + '</p><h2>stdout</h2><pre>' + (data.stdout || '') + '</pre><h2>stderr</h2><pre>' + (data.stderr || '') + '</pre>';
        }
      })
      .catch(function (err) {
        if (mdContent) mdContent.innerHTML = '<p style="color:#f09090">Run failed: ' + err.message + '</p>';
      });
  }

  // ── Settings popup ──────────────────────────────────────────────────────────

  var PROVIDER_DEFAULTS = {
  anthropic:  { env: 'ANTHROPIC_API_KEY',  hasSubscription: true  },
  openai:     { env: 'OPENAI_API_KEY',     hasSubscription: false },
  grok:       { env: 'XAI_API_KEY',        hasSubscription: false },
  openrouter: { env: 'OPENROUTER_API_KEY', hasSubscription: false },
  minimax:    { env: 'MINIMAX_API_KEY',    hasSubscription: false }
  };
  var PROVIDER_NAMES = ['anthropic', 'openai', 'grok', 'openrouter', 'minimax'];

  function openSettings() {
    closeUserMenu();
    var bd = document.getElementById('settingsBackdrop');
    if (!bd) return;
    bd.classList.add('open');
    document.getElementById('settingsError').textContent = '';
    fetch('/api/settings/providers')
      .then(function (r) { return r.json(); })
      .then(function (data) {
        if (Array.isArray(data)) renderProviderCards(data);
        else renderProviderCards([]);
      })
      .catch(function () { renderProviderCards([]); });
  }

  function closeSettings() {
    var bd = document.getElementById('settingsBackdrop');
    if (bd) bd.classList.remove('open');
  }

  function renderProviderCards(providers) {
    var main = document.getElementById('settingsMain');
    if (!main) return;
    var html = '<div class="settings-section-title">LLM Providers</div>';
    providers.forEach(function (p, i) { html += buildCardHtml(p, i); });
    html += '<div class="provider-add" onclick="window.__studio.addProviderCard()">+ Add Provider</div>';
    main.innerHTML = html;
    // Check env vars for all cards
    providers.forEach(function (p, i) {
      checkEnvStatus(p.api_key_env, 'envStatus-' + i);
    });
  }

  function buildCardHtml(p, idx) {
    var kind = (p.provider || 'anthropic').toLowerCase();
    var def = PROVIDER_DEFAULTS[kind] || PROVIDER_DEFAULTS.anthropic;
    var isPrimary = (p.role || 'primary') === 'primary';
    var authMode = p.auth_token_env ? 'subscription' : 'apikey';

    var h = '<div class="provider-card' + (isPrimary ? ' primary' : '') + '" data-idx="' + idx + '">';
    // Header row
    h += '<div class="pc-header">';
    h += '<select class="pc-select" onchange="window.__studio.onProviderChange(' + idx + ',this.value)">';
    PROVIDER_NAMES.forEach(function (n) {
      h += '<option value="' + n + '"' + (n === kind ? ' selected' : '') + '>' + n.charAt(0).toUpperCase() + n.slice(1) + '</option>';
    });
    h += '</select>';
    h += '<span class="pc-role ' + (isPrimary ? 'pc-role-primary' : '') + '">' + (isPrimary ? 'PRIMARY' : 'FALLBACK') + '</span>';
    h += '<span class="pc-remove" onclick="window.__studio.removeProviderCard(' + idx + ')" title="Remove">\u00d7</span>';
    h += '</div>';

    // Auth mode (subscription only for anthropic)
    if (def.hasSubscription) {
      h += '<div class="pc-row"><span class="pc-label">Auth</span>';
      h += '<label class="pc-radio"><input type="radio" name="auth-' + idx + '" value="apikey"' + (authMode === 'apikey' ? ' checked' : '') + ' onchange="window.__studio.onAuthChange(' + idx + ',\'apikey\')"/> API Key</label>';
      h += '<label class="pc-radio"><input type="radio" name="auth-' + idx + '" value="subscription"' + (authMode === 'subscription' ? ' checked' : '') + ' onchange="window.__studio.onAuthChange(' + idx + ',\'subscription\')"/> Subscription</label>';
      h += '</div>';
    }

    // Env var
    var envLabel = authMode === 'subscription' ? 'Token env' : 'API Key env';
    var envVal = authMode === 'subscription' ? (p.auth_token_env || '') : (p.api_key_env || def.env);
    h += '<div class="pc-row"><span class="pc-label">' + envLabel + '</span>';
    h += '<input class="pc-input pc-env" id="pcEnv-' + idx + '" value="' + envVal + '" placeholder="ENV_VAR_NAME" onblur="window.__studio.checkEnvStatus(this.value,\'envStatus-' + idx + '\')"/>';
    h += '<span class="env-status" id="envStatus-' + idx + '"></span></div>';

    // Role toggle
    h += '<div class="pc-row"><span class="pc-label">Role</span>';
    h += '<button class="pc-role-btn' + (isPrimary ? ' active' : '') + '" onclick="window.__studio.onRoleChange(' + idx + ',\'primary\')">Primary</button>';
    h += '<button class="pc-role-btn' + (!isPrimary ? ' active' : '') + '" onclick="window.__studio.onRoleChange(' + idx + ',\'fallback\')">Fallback</button>';
    h += '</div>';

    h += '</div>';
    return h;
  }

  function addProviderCard() {
    var main = document.getElementById('settingsMain');
    if (!main) return;
    var cards = main.querySelectorAll('.provider-card');
    var idx = cards.length;
    var addBtn = main.querySelector('.provider-add');
    var newCard = document.createElement('div');
    newCard.innerHTML = buildCardHtml({
      provider: 'anthropic', role: 'fallback',
      api_key_env: 'ANTHROPIC_API_KEY', auth_token_env: null
    }, idx);
    main.insertBefore(newCard.firstChild, addBtn);
    checkEnvStatus('ANTHROPIC_API_KEY', 'envStatus-' + idx);
  }

  function removeProviderCard(idx) {
    var card = qs('.provider-card[data-idx="' + idx + '"]');
    if (card) card.remove();
    // Re-index remaining cards
    var cards = qsa('.provider-card');
    cards.forEach(function (c, i) { c.dataset.idx = i; });
  }

  function onProviderChange(idx, kind) {
    var def = PROVIDER_DEFAULTS[kind] || PROVIDER_DEFAULTS.anthropic;
    var envEl = document.getElementById('pcEnv-' + idx);
    if (envEl) envEl.value = def.env;
    // Re-render card to show/hide subscription option
    var cards = collectProviders();
    if (cards[idx]) {
      cards[idx].provider = kind;
      cards[idx].api_key_env = def.env;
      cards[idx].auth_token_env = null;
    }
    renderProviderCards(cards);
  }

  function onAuthChange(idx, mode) {
    var def = PROVIDER_DEFAULTS.anthropic;
    var envEl = document.getElementById('pcEnv-' + idx);
    if (mode === 'subscription') {
      if (envEl) envEl.value = 'ANTHROPIC_AUTH_TOKEN';
    } else {
      if (envEl) envEl.value = def.env;
    }
    checkEnvStatus(envEl ? envEl.value : '', 'envStatus-' + idx);
  }

  function onRoleChange(idx, role) {
    // Collect, set this one, demote others if primary
    var cards = collectProviders();
    cards.forEach(function (c, i) {
      if (role === 'primary') c.role = (i === idx) ? 'primary' : 'fallback';
      else if (i === idx) c.role = 'fallback';
    });
    renderProviderCards(cards);
  }

  function collectProviders() {
    var cards = qsa('.provider-card');
    var result = [];
    cards.forEach(function (card, i) {
      var selectEl = card.querySelector('.pc-select');
      var envEl = document.getElementById('pcEnv-' + i);
      var roleEl = card.querySelector('.pc-role');
      var authRadio = card.querySelector('input[name="auth-' + i + '"]:checked');
      var kind = selectEl ? selectEl.value : 'anthropic';
      var authMode = authRadio ? authRadio.value : 'apikey';
      result.push({
        provider: kind,
        role: roleEl && roleEl.textContent === 'PRIMARY' ? 'primary' : 'fallback',
        api_key_env: authMode === 'apikey' ? (envEl ? envEl.value : '') : (PROVIDER_DEFAULTS[kind] || {}).env || '',
        auth_token_env: authMode === 'subscription' ? (envEl ? envEl.value : '') : null,
        base_url: null
      });
    });
    return result;
  }

  function saveProviders() {
    var providers = collectProviders();
    var btn = document.getElementById('settingsSaveBtn');
    var errEl = document.getElementById('settingsError');
    if (btn) { btn.disabled = true; btn.textContent = 'Saving\u2026'; }
    if (errEl) errEl.textContent = '';

    fetch('/api/settings/providers', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(providers)
    })
    .then(function (r) { return r.json(); })
    .then(function (data) {
      if (btn) { btn.disabled = false; btn.textContent = 'Save'; }
      if (data.error) {
        if (errEl) { errEl.textContent = data.error; errEl.style.color = '#f09090'; }
      } else {
        if (errEl) { errEl.textContent = 'Saved \u2713'; errEl.style.color = '#6fd8b2'; }
        setTimeout(function () { if (errEl) errEl.textContent = ''; }, 3000);
      }
    })
    .catch(function (err) {
      if (btn) { btn.disabled = false; btn.textContent = 'Save'; }
      if (errEl) { errEl.textContent = err.message; errEl.style.color = '#f09090'; }
    });
  }

  function checkEnvStatus(envVar, statusElId) {
    var el = document.getElementById(statusElId);
    if (!el || !envVar) { if (el) el.innerHTML = ''; return; }
    fetch('/api/settings/check-env?var=' + encodeURIComponent(envVar))
      .then(function (r) { return r.json(); })
      .then(function (data) {
        if (data.set) {
          el.innerHTML = '<span style="color:#6fd8b2">\u2713 set</span>';
        } else {
          el.innerHTML = '<span style="color:#e07830">\u26A0 not set</span>';
        }
      })
      .catch(function () { el.innerHTML = ''; });
  }

  // ── Agent Templates popup ──────────────────────────────────────────────────────

  function openAgentTemplates() {
    closeAllPopups();
    fetch('/api/agent_templates')
      .then(function (r) { return r.json(); })
      .then(function (templates) {
        var bd = document.getElementById('settingsBackdrop');
        var popup = document.getElementById('settingsPopup');
        if (!bd || !popup) return;
        popup.innerHTML = '';

        var header = document.createElement('div');
        header.className = 'sp-header';
        var title = document.createElement('h2');
        title.textContent = 'Agent Templates';
        header.appendChild(title);
        var closeBtn = document.createElement('button');
        closeBtn.className = 'sp-close';
        closeBtn.textContent = '\u00d7';
        closeBtn.onclick = function () { window.__studio.closeSettings(); };
        header.appendChild(closeBtn);
        popup.appendChild(header);

        var body = document.createElement('div');
        body.className = 'sp-body';
        templates.forEach(function (t) {
          var card = document.createElement('div');
          card.style.cssText = 'background:#1a1d1f;border-radius:8px;padding:12px;margin-bottom:8px';
          var row = document.createElement('div');
          row.style.cssText = 'display:flex;justify-content:space-between;align-items:center';
          var nameEl = document.createElement('strong');
          nameEl.style.color = '#e8e4d9';
          nameEl.textContent = t.name || '';
          var roleEl = document.createElement('span');
          roleEl.className = 'tree-badge tb-mod';
          roleEl.textContent = t.role || '';
          row.appendChild(nameEl);
          row.appendChild(roleEl);
          card.appendChild(row);
          var details = document.createElement('div');
          details.style.cssText = 'color:#908c82;font-size:12px;margin-top:4px';
          details.textContent = (t.tools_count || 0) + ' tools \u00b7 ' + (Array.isArray(t.specialization) ? t.specialization.join(', ') : '');
          card.appendChild(details);
          var pre = document.createElement('pre');
          pre.style.cssText = 'color:#5a5855;font-size:11px;margin-top:6px;white-space:pre-wrap;max-height:80px;overflow:auto';
          pre.textContent = (t.prompt_preview || '') + '\u2026';
          card.appendChild(pre);
          body.appendChild(card);
        });
        popup.appendChild(body);
        bd.classList.add('open');
      })
      .catch(function (e) { console.error('Agent templates error:', e); });
  }

  // ── Command palette ───────────────────────────────────────────────────────────

  function openSearch() {
    var bd = document.getElementById('cmdBackdrop');
    if (bd) bd.classList.add('open');
    setTimeout(function () {
      var inp = document.getElementById('cmdInput');
      if (inp) inp.focus();
    }, 50);
    filterCmd();
  }

  function closeSearch() {
    var bd  = document.getElementById('cmdBackdrop');
    var inp = document.getElementById('cmdInput');
    if (bd)  bd.classList.remove('open');
    if (inp) inp.value = '';
  }

  function closeCmdIfOutside(e) {
    var bd = document.getElementById('cmdBackdrop');
    if (bd && e.target === bd) closeSearch();
  }

  function filterCmd() {
    var inp = document.getElementById('cmdInput');
    var q   = inp ? inp.value.toLowerCase().trim() : '';
    qsa('.cmd-item').forEach(function (item) {
      var text = (item.textContent + ' ' + (item.dataset.filter || '')).toLowerCase();
      item.style.display = (!q || text.indexOf(q) !== -1) ? 'flex' : 'none';
    });
    qsa('.cmd-group-label').forEach(function (lbl) {
      var next    = lbl.nextElementSibling;
      var anyShown = false;
      while (next && !next.classList.contains('cmd-group-label')) {
        if (next.style.display !== 'none') anyShown = true;
        next = next.nextElementSibling;
      }
      lbl.style.display = anyShown ? '' : 'none';
    });
  }

  // Command palette item click dispatch.
  document.addEventListener('click', function (e) {
    var item = e.target.closest('.cmd-item');
    if (!item) return;
    var filter = item.dataset.filter || '';
    closeSearch();
    if (filter.indexOf('new intent') !== -1) { openCreateIntent(); }
    else if (filter.indexOf('build') !== -1) { toggleFunction('build'); }
    else if (filter.indexOf('theme') !== -1 && window.__studio) { /* theme toggle handled elsewhere */ }
    else if (filter.indexOf('provider') !== -1) { openSettings(); }
    else if (filter.indexOf('registry') !== -1) { toggleFunction('intents'); }
    else if (filter.indexOf('agent template') !== -1) { openAgentTemplates(); }
  });

  // ── Popups ────────────────────────────────────────────────────────────────────

  function openPopup(name) {
    closeAllPopups();
    var el = document.getElementById('popup-' + name);
    var bd = document.getElementById('backdrop');
    if (el) el.classList.add('open');
    if (bd) bd.classList.add('open');
  }

  function closeAllPopups() {
    qsa('.overlay').forEach(function (o) { o.classList.remove('open'); });
    var bd = document.getElementById('backdrop');
    if (bd) bd.classList.remove('open');
  }

  function toggleUserMenu(e) {
    if (e) e.stopPropagation();
    var um = document.getElementById('userMenu');
    if (um) um.classList.toggle('open');
  }

  function closeUserMenu() {
    var um = document.getElementById('userMenu');
    if (um) um.classList.remove('open');
  }

  // ── Click-outside handlers ────────────────────────────────────────────────────

  document.addEventListener('click', function (e) {
    // Close unpinned sidebar when clicking outside
    if (sidebarOpen && !isPinned) {
      var sb  = document.getElementById('sidebar');
      var rl  = document.getElementById('iconRail');
      var hd  = qs('header');
      var ft  = qs('.footer');
      if (sb && rl && hd && ft &&
          !sb.contains(e.target) &&
          !rl.contains(e.target) &&
          !hd.contains(e.target) &&
          !ft.contains(e.target)) {
        closeSidebarUI();
      }
    }

    // Close user menu
    var menu = document.getElementById('userMenu');
    var ab   = document.getElementById('avatarBtn');
    if (menu && ab && !menu.contains(e.target) && !ab.contains(e.target)) {
      closeUserMenu();
    }

    // Close filter popup when clicking outside
    if (filterPopupVisible) {
      var popup = qs('.filter-popup');
      var fBtn  = qs('.filter-toggle-btn');
      if (popup && !popup.contains(e.target) && fBtn && !fBtn.contains(e.target)) {
        popup.remove();
        filterPopupVisible = false;
      }
    }
  });

  // ── Keyboard shortcuts ────────────────────────────────────────────────────────

  document.addEventListener('keydown', function (e) {
    // Cmd/Ctrl+K → toggle command palette
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault();
      var bd = document.getElementById('cmdBackdrop');
      if (bd && bd.classList.contains('open')) closeSearch();
      else openSearch();
      return;
    }

    if (e.key === 'Escape') {
      // Close settings popup first
      var stBd = document.getElementById('settingsBackdrop');
      if (stBd && stBd.classList.contains('open')) { closeSettings(); return; }
      // Close create-intent popup
      var cipBd = document.getElementById('cipBackdrop');
      if (cipBd && cipBd.classList.contains('open')) { closeCreateIntent(); return; }
      // Close command palette
      var cmdBd = document.getElementById('cmdBackdrop');
      if (cmdBd && cmdBd.classList.contains('open')) { closeSearch(); return; }
      // Close other popups/menus
      closeAllPopups();
      closeUserMenu();
      if (sidebarOpen && !isPinned) closeSidebarUI();
      return;
    }

    // '?' → shortcuts overlay (only when not in a text field)
    if (e.key === '?' && !e.ctrlKey && !e.metaKey) {
      var active = document.activeElement;
      if (active && (active.tagName === 'INPUT' || active.tagName === 'TEXTAREA')) return;
      openPopup('shortcuts');
    }
  });

  // ── Grid & snap ───────────────────────────────────────────────────────────────

  var GRID_BASE = 12;
  var SNAP_STEP = GRID_BASE;

  function snapToGrid(val) {
    return Math.round(val / SNAP_STEP) * SNAP_STEP;
  }

  function ensureDotGridPattern() {
    if (!svgCanvas) return;
    var defs = svgCanvas.querySelector('defs');
    if (!defs) return;
    if (defs.querySelector('#dot-grid-pattern')) return;
    var ns      = 'http://www.w3.org/2000/svg';
    var pattern = document.createElementNS(ns, 'pattern');
    pattern.setAttribute('id', 'dot-grid-pattern');
    pattern.setAttribute('width', GRID_BASE);
    pattern.setAttribute('height', GRID_BASE);
    pattern.setAttribute('patternUnits', 'userSpaceOnUse');
    var dot = document.createElementNS(ns, 'circle');
    dot.setAttribute('cx', GRID_BASE / 2);
    dot.setAttribute('cy', GRID_BASE / 2);
    dot.setAttribute('r', '0.8');
    dot.setAttribute('class', 'dot-grid-dot');
    pattern.appendChild(dot);
    defs.appendChild(pattern);
  }

  function addDotGridBackground() {
    if (!svgCanvas) return;
    var g = svgCanvas.querySelector('g');
    if (!g) return;
    if (g.querySelector('.dot-grid-bg')) return;
    var ns   = 'http://www.w3.org/2000/svg';
    var rect = document.createElementNS(ns, 'rect');
    rect.setAttribute('class', 'dot-grid-bg');
    rect.setAttribute('x', '-50000');
    rect.setAttribute('y', '-50000');
    rect.setAttribute('width', '100000');
    rect.setAttribute('height', '100000');
    rect.setAttribute('fill', 'url(#dot-grid-pattern)');
    rect.setAttribute('pointer-events', 'none');
    g.insertBefore(rect, g.firstChild);
  }

  // No-op: grid alignment handled by SVG pattern
  function updateGrid() {}

  // ── SVG graph rendering ───────────────────────────────────────────────────────

  function renderSingleNode(g, node) {
    var group = document.createElementNS('http://www.w3.org/2000/svg', 'g');
    group.setAttribute('class', 'graph-node node-' + (node.node_type || 'default'));
    group.style.cursor = 'pointer';
    group.dataset.nodeId   = node.id;
    group.dataset.nodeType = node.node_type;

    if (node.node_type && node.node_type.indexOf('entry') !== -1) {
      group.dataset.entry = 'true';
    }
    if (node.node_type && node.node_type.indexOf('exit') !== -1) {
      group.dataset.exit = 'true';
    }

    var nx = snapToGrid(node.x);
    var ny = snapToGrid(node.y);

    var rect = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
    rect.setAttribute('x', nx - node.width / 2);
    rect.setAttribute('y', ny - node.height / 2);
    rect.setAttribute('width',  node.width);
    rect.setAttribute('height', node.height);

    var rx = '8';
    switch (node.node_type) {
      case 'person':    rx = String(node.width / 2); break;
      case 'system':
      case 'container':
      case 'component': rx = '12'; break;
      case 'boundary':  rx = '16'; break;
      case 'external':  rx = '2';  break;
      case 'component-dead':
      case 'component-sub': rx = '8'; break;
      case 'block':     rx = '4';  break;
      case 'Const':
      case 'ConstF64':
      case 'ConstBool': rx = String(node.width / 2); break;
    }
    rect.setAttribute('rx', rx);
    rect.setAttribute('ry', rx);
    rect.setAttribute('class', 'node-rect');
    group.appendChild(rect);

    var label = document.createElementNS('http://www.w3.org/2000/svg', 'text');
    label.setAttribute('x', nx);
    label.setAttribute('y', node.badge ? ny - 4 : ny + 4);
    label.setAttribute('text-anchor', 'middle');
    label.setAttribute('class', 'node-label');
    label.textContent = node.label;
    group.appendChild(label);

    if (node.badge) {
      var badge = document.createElementNS('http://www.w3.org/2000/svg', 'text');
      badge.setAttribute('x', nx);
      badge.setAttribute('y', ny + 14);
      badge.setAttribute('text-anchor', 'middle');
      badge.setAttribute('class', 'node-badge');
      badge.textContent = node.badge;
      group.appendChild(badge);
    }

    if (node.node_type && node.node_type.indexOf('entry') !== -1) {
      var marker = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
      marker.setAttribute('cx', nx - node.width / 2 + 10);
      marker.setAttribute('cy', ny - node.height / 2 + 10);
      marker.setAttribute('r', '5');
      marker.setAttribute('fill', '#3fb950');
      marker.setAttribute('class', 'entry-marker');
      group.appendChild(marker);
      var mt = document.createElementNS('http://www.w3.org/2000/svg', 'text');
      mt.setAttribute('x', nx - node.width / 2 + 20);
      mt.setAttribute('y', ny - node.height / 2 + 14);
      mt.setAttribute('class', 'node-badge');
      mt.setAttribute('fill', '#3fb950');
      mt.setAttribute('font-size', '9');
      mt.textContent = 'IN';
      group.appendChild(mt);
    }

    if (node.node_type && node.node_type.indexOf('exit') !== -1) {
      var emarker = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
      emarker.setAttribute('cx', nx - node.width / 2 + 10);
      emarker.setAttribute('cy', ny - node.height / 2 + 10);
      emarker.setAttribute('r', '5');
      emarker.setAttribute('fill', '#d29922');
      emarker.setAttribute('class', 'exit-marker');
      group.appendChild(emarker);
      var et = document.createElementNS('http://www.w3.org/2000/svg', 'text');
      et.setAttribute('x', nx - node.width / 2 + 20);
      et.setAttribute('y', ny - node.height / 2 + 14);
      et.setAttribute('class', 'node-badge');
      et.setAttribute('fill', '#d29922');
      et.setAttribute('font-size', '9');
      et.textContent = 'OUT';
      group.appendChild(et);
    }

    group.addEventListener('click',    function () { onNodeClick(node); });
    group.addEventListener('dblclick', function () { onNodeDblClick(node); });

    g.appendChild(group);
  }

  function renderGraph(data) {
    lastGraphData = data;
    var svg = qs('.graph-canvas');
    if (!svg) return;
    var g = svg.querySelector('g');
    if (!g) return;

    g.innerHTML = '';
    ensureDotGridPattern();
    addDotGridBackground();

    if (data.bbox) {
      var b   = data.bbox;
      var pad = 40;
      svg.setAttribute('viewBox',
        (b.min_x - pad) + ' ' + (b.min_y - pad) + ' ' +
        (b.max_x - b.min_x + 2 * pad) + ' ' + (b.max_y - b.min_y + 2 * pad));
    }

    // Render edges first
    if (data.edges) {
      data.edges.forEach(function (edge) {
        if (edge.source === edge.target) return;

        var pathEl = document.createElementNS('http://www.w3.org/2000/svg', 'path');
        pathEl.setAttribute('d', edge.path_data);
        pathEl.setAttribute('class', 'edge-path edge-' + (edge.edge_type || 'default'));
        pathEl.setAttribute('marker-end', 'url(#arrowhead)');
        pathEl.dataset.edgeSrc = edge.source;
        pathEl.dataset.edgeTgt = edge.target;
        g.appendChild(pathEl);

        if (edge.label && edge.label_x) {
          var text = document.createElementNS('http://www.w3.org/2000/svg', 'text');
          text.setAttribute('x', edge.label_x);
          text.setAttribute('y', edge.label_y);
          text.setAttribute('text-anchor', 'middle');
          text.setAttribute('dominant-baseline', 'central');
          text.setAttribute('class', 'edge-label');
          text.textContent = edge.label;
          g.appendChild(text);
        }
      });
    }

    // Separate boundary nodes from regular nodes
    var boundaryNodes = [];
    var regularNodes  = [];
    if (data.nodes) {
      data.nodes.forEach(function (node) {
        if (node.node_type === 'boundary') boundaryNodes.push(node);
        else regularNodes.push(node);
      });
    }

    regularNodes.forEach(function (node) { renderSingleNode(g, node); });

    // Render boundary nodes sized to enclose children
    boundaryNodes.forEach(function (bNode) {
      var childIds = [];
      regularNodes.forEach(function (n) {
        if (n.node_type === 'container') childIds.push(n.id);
      });

      var bPad = 40;
      var minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
      childIds.forEach(function (cid) {
        var el = qs('[data-node-id="' + cid + '"]');
        if (!el) return;
        var r = el.querySelector('.node-rect');
        if (!r) return;
        var rx = parseFloat(r.getAttribute('x'));
        var ry = parseFloat(r.getAttribute('y'));
        var rw = parseFloat(r.getAttribute('width'));
        var rh = parseFloat(r.getAttribute('height'));
        if (rx < minX)      minX = rx;
        if (ry < minY)      minY = ry;
        if (rx + rw > maxX) maxX = rx + rw;
        if (ry + rh > maxY) maxY = ry + rh;
      });

      if (minX === Infinity) {
        renderSingleNode(g, bNode);
        return;
      }

      var bx = minX - bPad;
      var by = minY - bPad - 28;
      var bw = maxX - minX + 2 * bPad;
      var bh = maxY - minY + 2 * bPad + 28;

      var bGroup = document.createElementNS('http://www.w3.org/2000/svg', 'g');
      bGroup.setAttribute('class', 'graph-node node-boundary');
      bGroup.style.cursor     = 'pointer';
      bGroup.dataset.nodeId   = bNode.id;
      bGroup.dataset.nodeType = 'boundary';
      bGroup.dataset.children = childIds.join(',');

      var bRect = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
      bRect.setAttribute('x',      bx);
      bRect.setAttribute('y',      by);
      bRect.setAttribute('width',  bw);
      bRect.setAttribute('height', bh);
      bRect.setAttribute('rx', '16');
      bRect.setAttribute('ry', '16');
      bRect.setAttribute('class', 'node-rect');
      bGroup.appendChild(bRect);

      var lbl = document.createElementNS('http://www.w3.org/2000/svg', 'text');
      lbl.setAttribute('x', bx + 16);
      lbl.setAttribute('y', by + 20);
      lbl.setAttribute('text-anchor', 'start');
      lbl.setAttribute('class', 'node-label boundary-label');
      lbl.textContent = bNode.label;
      bGroup.appendChild(lbl);

      bGroup.addEventListener('click',    function () { onNodeClick(bNode); });
      bGroup.addEventListener('dblclick', function () { onNodeDblClick(bNode); });

      var firstChild = qs('[data-node-id="' + childIds[0] + '"]');
      if (firstChild) g.insertBefore(bGroup, firstChild);
      else g.appendChild(bGroup);

      nodePositions[bNode.id] = {
        cx: bx + bw / 2, cy: by + bh / 2, w: bw, h: bh
      };
    });

    // Push external nodes away from boundary rects
    boundaryNodes.forEach(function (bNode) {
      var bEl = qs('[data-node-id="' + bNode.id + '"]');
      if (!bEl) return;
      var bRect = bEl.querySelector('.node-rect');
      if (!bRect) return;
      var bbx = parseFloat(bRect.getAttribute('x'));
      var bby = parseFloat(bRect.getAttribute('y'));
      var bbw = parseFloat(bRect.getAttribute('width'));
      var bbh = parseFloat(bRect.getAttribute('height'));
      var MARGIN = 30;

      regularNodes.forEach(function (n) {
        if (n.node_type === 'container') return;
        var nEl = qs('[data-node-id="' + n.id + '"]');
        if (!nEl) return;
        var nRect = nEl.querySelector('.node-rect');
        if (!nRect) return;
        var nx = parseFloat(nRect.getAttribute('x'));
        var ny = parseFloat(nRect.getAttribute('y'));
        var nw = parseFloat(nRect.getAttribute('width'));
        var nh = parseFloat(nRect.getAttribute('height'));

        var hOverlap = nx < bbx + bbw && nx + nw > bbx;
        if (!hOverlap) return;

        if (ny + nh > bby - MARGIN && ny + nh <= bby + bbh / 2) {
          var shiftY = (bby - MARGIN) - (ny + nh);
          nRect.setAttribute('y', ny + shiftY);
          nEl.querySelectorAll('text').forEach(function (lbEl) {
            lbEl.setAttribute('y', parseFloat(lbEl.getAttribute('y')) + shiftY);
          });
        }
        if (ny < bby + bbh + MARGIN && ny >= bby + bbh / 2) {
          var newY   = bby + bbh + MARGIN;
          var shiftY2 = newY - ny;
          nRect.setAttribute('y', ny + shiftY2);
          nEl.querySelectorAll('text').forEach(function (lbEl2) {
            lbEl2.setAttribute('y', parseFloat(lbEl2.getAttribute('y')) + shiftY2);
          });
        }
      });
    });

    storeNodePositions();
    updateEdges();

    // Cache C4 hierarchy data for sidebar tree
    if (data.modules) sidebarModules = data.modules;

    if (currentLevel === 'context' && data.nodes) {
      sidebarModules.forEach(function (mod) {
        sidebarContextNodes[mod] = data.nodes
          .filter(function (n) { return n.node_type !== 'boundary' && n.node_type !== 'system'; })
          .map(function (n) { return { id: n.id, label: n.label, type: n.node_type }; });
      });
    }
    if (currentLevel === 'container' && data.nodes) {
      sidebarContainers[currentModule] = data.nodes
        .filter(function (n) { return n.node_type !== 'boundary'; })
        .map(function (n) { return { id: n.id, label: n.label, type: n.node_type }; });
    }
    if (currentLevel === 'component' && data.nodes) {
      sidebarComponents[currentModule] = data.nodes
        .filter(function (n) { return n.node_type !== 'boundary'; })
        .map(function (n) { return { id: n.id, label: n.label, type: n.node_type }; });
    }
    if (currentLevel === 'code' && data.nodes) {
      var codeKey = currentModule + '/' + currentFunction + '/' + currentBlock;
      sidebarCodeOps[codeKey] = data.nodes
        .map(function (n) { return { id: n.id, label: n.label }; });
    }

    updateSidebarTree();
    applyFilters();
  }

  // ── Inspector panel ───────────────────────────────────────────────────────────

  function updateInspector(node) {
    var inspector = qs('.inspector-panel');
    if (!inspector) return;
    inspector.innerHTML =
      '<h3 class="panel-title">Inspector</h3>' +
      '<div class="inspector-content">' +
        '<div class="inspector-field"><span class="field-label">@id</span><span class="field-value">'   + node.id        + '</span></div>' +
        '<div class="inspector-field"><span class="field-label">@type</span><span class="field-value">' + node.node_type + '</span></div>' +
        '<div class="inspector-field"><span class="field-label">label</span><span class="field-value">' + node.label     + '</span></div>' +
        (node.badge ? '<div class="inspector-field"><span class="field-label">info</span><span class="field-value">' + node.badge + '</span></div>' : '') +
      '</div>';
    qsa('.graph-node').forEach(function (g) { g.classList.remove('selected'); });
    var target = qs('[data-node-id="' + node.id + '"]');
    if (target) target.classList.add('selected');
  }

  // ── Node click / dblclick handlers ────────────────────────────────────────────

  function onNodeClick(node) {
    updateInspector(node);
  }

  function onNodeDblClick(node) {
    if (currentLevel === 'context' && (node.node_type === 'system' || node.node_type === 'module')) {
      navigateTo('container', 'app/main');
    } else if (currentLevel === 'container' && node.id === 'container:binary') {
      navigateTo('component', currentModule || 'app/main', 'main');
    } else if (currentLevel === 'container' && node.node_type === 'function') {
      navigateTo('component', currentModule || 'app/main', node.id);
    } else if (currentLevel === 'component' && (node.node_type === 'component' || node.node_type === 'block')) {
      var fnName = node.id.replace(/^component:/, '');
      navigateTo('code', currentModule || 'app/main', fnName, 'entry');
    }
  }

  // ── Layout persistence across navigation ──────────────────────────────────────

  var savedLayouts = {};

  function viewKey(level, mod, fn, blk) {
    var parts = [level || currentLevel];
    if (mod || currentModule)   parts.push(mod || currentModule);
    if (fn  || currentFunction) parts.push(fn  || currentFunction);
    if (blk || currentBlock)    parts.push(blk || currentBlock);
    parts.push(currentLayout || 'hierarchical');
    return parts.join('/');
  }

  function saveCurrentLayout() {
    var key = viewKey();
    savedLayouts[key] = {
      positions: JSON.parse(JSON.stringify(nodePositions)),
      panX:  svgPanX,
      panY:  svgPanY,
      scale: svgScale
    };
  }

  function applyLayoutIfSaved(key) {
    var saved = savedLayouts[key];
    if (!saved) return false;

    svgPanX  = saved.panX;
    svgPanY  = saved.panY;
    svgScale = saved.scale;
    updateSvgTransform();

    qsa('.graph-node').forEach(function (grp) {
      var nodeId = grp.dataset.nodeId;
      var pos    = saved.positions[nodeId];
      if (!pos) return;
      var r = grp.querySelector('.node-rect');
      if (!r) return;
      var w = pos.w, h = pos.h, cx = pos.cx, cy = pos.cy;

      grp.querySelectorAll('*').forEach(function (el) { el.style.transition = 'none'; });

      r.setAttribute('x', cx - w / 2);
      r.setAttribute('y', cy - h / 2);

      var isBnd = grp.dataset.nodeType === 'boundary';
      grp.querySelectorAll('text').forEach(function (t) {
        if (t.classList.contains('node-label')) {
          if (isBnd) {
            t.setAttribute('x', cx - w / 2 + 16);
            t.setAttribute('y', cy - h / 2 + 20);
          } else {
            t.setAttribute('x', cx);
            t.setAttribute('y', cy - 4);
          }
        } else if (t.classList.contains('node-badge') && !t.hasAttribute('font-size')) {
          t.setAttribute('x', cx);
          t.setAttribute('y', cy + 14);
        }
      });
      grp.querySelectorAll('circle').forEach(function (c) {
        if (c.classList.contains('entry-marker') || c.classList.contains('exit-marker')) {
          c.setAttribute('cx', cx - w / 2 + 10);
          c.setAttribute('cy', cy - h / 2 + 10);
        }
      });
      grp.querySelectorAll('text[font-size=\'9\']').forEach(function (t) {
        t.setAttribute('x', cx - w / 2 + 20);
        t.setAttribute('y', cy - h / 2 + 14);
      });
      nodePositions[nodeId] = { cx: cx, cy: cy, w: w, h: h };

      requestAnimationFrame(function () {
        grp.querySelectorAll('*').forEach(function (el) { el.style.transition = ''; });
      });
    });

    updateEdges();
    return true;
  }

  // ── Sidebar tree state ────────────────────────────────────────────────────────

  var sidebarModules      = [];
  var sidebarContextNodes = {};
  var sidebarContainers   = {};
  var sidebarComponents   = {};
  var sidebarCodeOps      = {};

  function c4Icon(type) {
    switch (type) {
      case 'person':           return { icon: '\uD83D\uDC64', color: '#5b9bd5' };
      case 'system':           return { icon: '\u2B22',       color: '#5b9bd5' };
      case 'external':         return { icon: '\u2B21',       color: '#999'    };
      case 'container:binary': return { icon: '\u25A0',       color: '#2ecc71' };
      case 'container':        return { icon: '\u25A1',       color: '#7f8c8d' };
      case 'component':        return { icon: '\u25C6',       color: '#3498db' };
      case 'component:dead':   return { icon: '\u25C7',       color: '#666'    };
      default:                 return { icon: '\u25B8',       color: '#888'    };
    }
  }

  var chevronRight = '<svg viewBox="0 0 16 16"><polyline points="6 4 10 8 6 12"/></svg>';
  var chevronDown  = '<svg viewBox="0 0 16 16"><polyline points="4 6 8 10 12 6"/></svg>';

  function sidebarItem(label, type, depth, isActive, onClick, expandable) {
    var li         = document.createElement('li');
    var depthClass = depth === 1 ? ' tree-child'
                   : depth === 2 ? ' tree-child tree-child-2'
                   : depth === 3 ? ' tree-child tree-child-2 tree-child-3'
                   : '';
    li.className = 'module-item' + depthClass + (isActive ? ' tree-active' : '');
    var ic        = c4Icon(type);
    var arrowHtml = expandable
      ? '<span class="tree-arrow">' + (isActive ? chevronDown : chevronRight) + '</span>'
      : '';
    li.innerHTML = arrowHtml +
      '<span class="tree-icon" style="color:' + ic.color + '">' + ic.icon + '</span>' +
      '<span class="module-name">' + label + '</span>';
    if (onClick) {
      li.style.cursor = 'pointer';
      li.addEventListener('click', onClick);
    } else {
      li.style.opacity = '0.5';
    }
    return li;
  }

  function updateSidebarTree() {
    var tree = qs('.module-tree');
    if (!tree) return;
    tree.innerHTML = '';

    sidebarModules.forEach(function (mod) {
      var isActiveModule = (currentModule === mod);
      var isExpanded     = isActiveModule && currentLevel !== 'context';

      var li = document.createElement('li');
      li.className = 'module-item' + (isActiveModule ? ' tree-active' : '');
      var arrow = isExpanded ? chevronDown : chevronRight;
      li.innerHTML =
        '<span class="tree-arrow">' + arrow + '</span>' +
        '<span class="tree-icon" style="color:#5b9bd5">\u2B22</span>' +
        '<span class="module-name">' + mod + '</span>';
      li.style.cursor = 'pointer';
      li.addEventListener('click', function () {
        if (isExpanded && currentLevel === 'container') navigateTo('context', mod);
        else navigateTo('container', mod);
      });
      tree.appendChild(li);

      if (!isExpanded) return;

      var containers   = sidebarContainers[mod] || [];

      containers.forEach(function (ct) {
        if (ct.id !== 'container:binary') return;

        var isActiveCt = (currentLevel === 'component' || currentLevel === 'code');

        tree.appendChild(sidebarItem(ct.label, ct.type, 1, isActiveCt, function () {
          if (isActiveCt && currentLevel === 'component') navigateTo('container', mod);
          else navigateTo('component', mod, 'main');
        }, true));

        if (isActiveCt && sidebarComponents[mod]) {
          sidebarComponents[mod]
            .filter(function (comp) { return comp.id === 'component:main'; })
            .forEach(function (comp) {
              var fnName      = comp.id.replace(/^component:/, '');
              var isActiveComp = (currentLevel === 'code' && currentFunction === fnName);

              tree.appendChild(sidebarItem(comp.label, 'component', 2, isActiveComp, function () {
                if (isActiveComp) navigateTo('component', mod, 'main');
                else navigateTo('code', mod, fnName, 'entry');
              }, false));
            });
        }
      });
    });
  }

  // ── Navigation ────────────────────────────────────────────────────────────────

  function navigateTo(level, module, func, block, restoreLayout) {
    if (restoreLayout !== false) saveCurrentLayout();

    currentLevel    = level;
    currentModule   = module || null;
    currentFunction = func   || null;
    currentBlock    = block  || null;

    qsa('.c4-tab').forEach(function (tab) {
      var tabLevel  = tab.textContent.toLowerCase();
      tab.classList.toggle('active', tabLevel === level);
      var reachable = tabLevel === 'context'
        || (tabLevel === 'container' && !!(module || currentModule))
        || (tabLevel === 'component' && !!(func   || currentFunction))
        || (tabLevel === 'code'      && !!(block  || currentBlock));
      tab.classList.toggle('disabled', !reachable);
      tab.disabled = !reachable;
    });

    updateBreadcrumb();

    var targetKey = viewKey(level, module, func, block);

    if (restoreLayout !== false) {
      var saved = savedLayouts[targetKey];
      if (saved) {
        svgPanX  = saved.panX;
        svgPanY  = saved.panY;
        svgScale = saved.scale;
      } else {
        svgScale = 1; svgPanX = 0; svgPanY = 0;
      }
    } else {
      svgScale = 1; svgPanX = 0; svgPanY = 0;
    }
    updateSvgTransform();

    var url    = '/api/graph/' + level;
    var params = [];
    if (module) params.push('module='   + encodeURIComponent(module));
    if (func)   params.push('function=' + encodeURIComponent(func));
    if (block)  params.push('block='    + encodeURIComponent(block));
    if (currentLayout && currentLayout !== 'hierarchical') {
      params.push('layout=' + currentLayout);
    }
    if (params.length) url += '?' + params.join('&');

    fetch(url)
      .then(function (r) { return r.json(); })
      .then(function (data) {
        if (data.error) { console.error('API error:', data.error); return; }
        renderGraph(data);
        if (restoreLayout !== false) applyLayoutIfSaved(targetKey);
        storeNodePositions();
        updateEdges();
      })
      .catch(function (e) { console.error('Fetch error:', e); });
  }

  // Track previously rendered node IDs to detect truly new nodes (#479).
  var _previousNodeIds = new Set();

  /** Reload the current graph view after a mutation (#479).
   *  Highlights changed node IDs with a glow animation and new nodes with fade-in. */
  function reloadCurrentGraph(changedNodeIds) {
    // Snapshot current node IDs before re-render.
    var prevIds = new Set(_previousNodeIds);

    var url    = '/api/graph/' + currentLevel;
    var params = [];
    if (currentModule)   params.push('module='   + encodeURIComponent(currentModule));
    if (currentFunction) params.push('function=' + encodeURIComponent(currentFunction));
    if (currentBlock)    params.push('block='    + encodeURIComponent(currentBlock));
    if (currentLayout && currentLayout !== 'hierarchical') {
      params.push('layout=' + currentLayout);
    }
    if (params.length) url += '?' + params.join('&');

    fetch(url)
      .then(function (r) { return r.json(); })
      .then(function (data) {
        if (data.error) { console.error('Graph refresh error:', data.error); return; }
        renderGraph(data);
        updateEdges();

        // Update tracked node IDs.
        _previousNodeIds = new Set();
        qsa('.graph-node').forEach(function (el) {
          var id = el.getAttribute('data-node-id');
          if (id) _previousNodeIds.add(id);
        });

        // Highlight changed nodes (modified in-place) with glow animation.
        if (changedNodeIds && changedNodeIds.length) {
          changedNodeIds.forEach(function (id) {
            var el = document.querySelector('[data-node-id="' + id + '"]');
            if (el) el.classList.add('node-changed');
          });
          setTimeout(function () {
            qsa('.node-changed').forEach(function (el) { el.classList.remove('node-changed'); });
          }, 2000);
        }

        // Mark truly new nodes (not in previous snapshot) with appear animation.
        qsa('.graph-node').forEach(function (el) {
          var id = el.getAttribute('data-node-id');
          if (id && !prevIds.has(id)) {
            el.classList.add('node-added');
          }
        });
      })
      .catch(function (e) { console.error('Graph refresh fetch error:', e); });
  }

  function updateBreadcrumb() {
    var nav = qs('.breadcrumb');
    if (!nav) return;

    var html = '<button class="breadcrumb-item" onclick="window.__studio.nav(\'context\')">workspace</button>';
    if (currentModule) {
      html += ' <span class="breadcrumb-sep">&gt;</span> ';
      html += '<button class="breadcrumb-item" onclick="window.__studio.nav(\'container\',\'' + currentModule + '\')">' + currentModule + '</button>';
    }
    if (currentFunction) {
      html += ' <span class="breadcrumb-sep">&gt;</span> ';
      html += '<button class="breadcrumb-item" onclick="window.__studio.nav(\'component\',\'' + currentModule + '\',\'' + currentFunction + '\')">' + currentFunction + '</button>';
    }
    if (currentBlock) {
      html += ' <span class="breadcrumb-sep">&gt;</span> ';
      html += '<span class="breadcrumb-item active">' + currentBlock + '</span>';
    }

    if (currentLevel === 'code' && currentBlock) {
      var toggleIcon = codeViewActive
        ? '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="8" cy="8" r="3"/><line x1="3" y1="3" x2="5.5" y2="5.5"/><line x1="13" y1="3" x2="10.5" y2="5.5"/><line x1="3" y1="13" x2="5.5" y2="10.5"/><line x1="13" y1="13" x2="10.5" y2="10.5"/></svg>'
        : '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"><polyline points="5,3 1,8 5,13"/><polyline points="11,3 15,8 11,13"/><line x1="10" y1="2" x2="6" y2="14"/></svg>';
      var toggleTitle = codeViewActive ? 'Switch to graph view' : 'Switch to code view';
      html += '<button class="breadcrumb-view-toggle" onclick="window.__studio.toggleCode()" title="' + toggleTitle + '">' + toggleIcon + '</button>';
    }

    nav.innerHTML = html;
  }

  // C4 tab clicks
  qsa('.c4-tab').forEach(function (tab) {
    tab.addEventListener('click', function () {
      var level = tab.textContent.toLowerCase();
      if (level === 'context') {
        navigateTo('context');
      } else if (level === 'container' && currentModule) {
        navigateTo('container', currentModule);
      } else if (level === 'component' && currentModule && currentFunction) {
        navigateTo('component', currentModule, currentFunction);
      } else if (level === 'code' && currentModule && currentFunction && currentBlock) {
        navigateTo('code', currentModule, currentFunction, currentBlock);
      }
    });
  });

  // ── SVG pan/zoom ──────────────────────────────────────────────────────────────

  var svgCanvas  = qs('.graph-canvas');
  var svgScale   = 1;
  var svgPanX    = 0, svgPanY = 0;
  var svgDragging = false;
  var svgLastX   = 0, svgLastY = 0;

  function updateSvgTransform() {
    if (!svgCanvas) return;
    var g = svgCanvas.querySelector('g');
    if (g) {
      g.setAttribute('transform',
        'translate(' + svgPanX + ',' + svgPanY + ') scale(' + svgScale + ')');
    }
    updateGrid();
  }

  function screenToSvg(clientX, clientY) {
    var ctm = svgCanvas.getScreenCTM();
    if (ctm) {
      var inv = ctm.inverse();
      return { x: inv.a * clientX + inv.c * clientY + inv.e,
               y: inv.b * clientX + inv.d * clientY + inv.f };
    }
    var rect = svgCanvas.getBoundingClientRect();
    return { x: clientX - rect.left, y: clientY - rect.top };
  }

  if (svgCanvas) {
    svgCanvas.addEventListener('wheel', function (e) {
      e.preventDefault();

      if (!e.ctrlKey) {
        // Two-finger scroll → pan
        var pt1 = screenToSvg(0, 0);
        var pt2 = screenToSvg(e.deltaX, e.deltaY);
        svgPanX -= (pt2.x - pt1.x);
        svgPanY -= (pt2.y - pt1.y);
        updateSvgTransform();
        return;
      }

      // Pinch/ctrl+scroll → zoom
      var factor   = 1 + Math.min(Math.abs(e.deltaY), 50) * 0.01;
      var delta    = e.deltaY > 0 ? 1 / factor : factor;
      var newScale = Math.max(0.2, Math.min(4, svgScale * delta));
      var mx       = screenToSvg(e.clientX, e.clientY);
      svgPanX  = mx.x - (mx.x - svgPanX) * (newScale / svgScale);
      svgPanY  = mx.y - (mx.y - svgPanY) * (newScale / svgScale);
      svgScale = newScale;
      updateSvgTransform();
    }, { passive: false });

    svgCanvas.addEventListener('mousedown', function (e) {
      if (e.target.closest('.graph-node')) return;
      svgDragging = true;
      svgLastX    = e.clientX;
      svgLastY    = e.clientY;
    });

    svgCanvas.addEventListener('mousemove', function (e) {
      if (!svgDragging) return;
      var pt1 = screenToSvg(svgLastX, svgLastY);
      var pt2 = screenToSvg(e.clientX, e.clientY);
      svgPanX  += (pt2.x - pt1.x);
      svgPanY  += (pt2.y - pt1.y);
      svgLastX  = e.clientX;
      svgLastY  = e.clientY;
      updateSvgTransform();
    });

    svgCanvas.addEventListener('mouseup',    function () { svgDragging = false; });
    svgCanvas.addEventListener('mouseleave', function () { svgDragging = false; });
  }

  // ── Node position tracking ────────────────────────────────────────────────────

  var nodePositions = {};

  function storeNodePositions() {
    nodePositions = {};
    qsa('.graph-node').forEach(function (g) {
      var r = g.querySelector('.node-rect');
      if (!r) return;
      var id = g.dataset.nodeId;
      var w  = parseFloat(r.getAttribute('width'));
      var h  = parseFloat(r.getAttribute('height'));
      nodePositions[id] = {
        cx: parseFloat(r.getAttribute('x')) + w / 2,
        cy: parseFloat(r.getAttribute('y')) + h / 2,
        w: w, h: h
      };
    });
  }

  function borderSide(nodePos, tx, ty) {
    var dx  = tx - nodePos.cx, dy = ty - nodePos.cy;
    var hw  = nodePos.w / 2,   hh = nodePos.h / 2;
    if (dx === 0 && dy === 0) return 'bottom';
    var absDx = Math.abs(dx), absDy = Math.abs(dy);
    if (absDx * hh > absDy * hw) return dx > 0 ? 'right' : 'left';
    return dy > 0 ? 'bottom' : 'top';
  }

  function distributedBorderPoint(nodePos, side, index, count) {
    var cx   = nodePos.cx, cy = nodePos.cy;
    var hw   = nodePos.w / 2, hh = nodePos.h / 2;
    var frac = (index + 1) / (count + 1);
    switch (side) {
      case 'top':    return { x: cx - hw + nodePos.w * frac, y: cy - hh };
      case 'bottom': return { x: cx - hw + nodePos.w * frac, y: cy + hh };
      case 'left':   return { x: cx - hw, y: cy - hh + nodePos.h * frac };
      case 'right':  return { x: cx + hw, y: cy - hh + nodePos.h * frac };
      default:       return { x: cx, y: cy + hh };
    }
  }

  function updateEdges() {
    var g = svgCanvas ? svgCanvas.querySelector('g') : null;
    if (!g) return;

    g.querySelectorAll('.conn-dot').forEach(function (d) { d.remove(); });

    var edgePaths = Array.prototype.slice.call(g.querySelectorAll('.edge-path'));
    var portMap   = {};
    var edgeInfo  = [];

    edgePaths.forEach(function (pathEl, i) {
      var srcId = pathEl.dataset.edgeSrc;
      var tgtId = pathEl.dataset.edgeTgt;
      if (srcId === tgtId) {
        pathEl.style.display = 'none';
        edgeInfo.push(null);
        return;
      }
      var src = nodePositions[srcId];
      var tgt = nodePositions[tgtId];
      if (!src || !tgt) { edgeInfo.push(null); return; }

      var srcSide = borderSide(src, tgt.cx, tgt.cy);
      var tgtSide = borderSide(tgt, src.cx, src.cy);
      var srcKey  = srcId + ':' + srcSide;
      var tgtKey  = tgtId + ':' + tgtSide;

      if (!portMap[srcKey]) portMap[srcKey] = [];
      if (!portMap[tgtKey]) portMap[tgtKey] = [];
      portMap[srcKey].push(i);
      portMap[tgtKey].push(i);

      edgeInfo.push({ srcId: srcId, tgtId: tgtId, srcSide: srcSide, tgtSide: tgtSide, srcKey: srcKey, tgtKey: tgtKey });
    });

    edgePaths.forEach(function (pathEl, i) {
      var info = edgeInfo[i];
      if (!info) return;
      var src = nodePositions[info.srcId];
      var tgt = nodePositions[info.tgtId];

      var srcSlots = portMap[info.srcKey];
      var tgtSlots = portMap[info.tgtKey];
      var srcIdx   = srcSlots.indexOf(i);
      var tgtIdx   = tgtSlots.indexOf(i);

      var sp   = distributedBorderPoint(src, info.srcSide, srcIdx, srcSlots.length);
      var tp   = distributedBorderPoint(tgt, info.tgtSide, tgtIdx, tgtSlots.length);
      var midY = (sp.y + tp.y) / 2;
      var d    = 'M ' + sp.x + ' ' + sp.y +
                 ' L ' + sp.x + ' ' + midY +
                 ' L ' + tp.x + ' ' + midY +
                 ' L ' + tp.x + ' ' + tp.y;
      pathEl.setAttribute('d', d);

      var labelEl = pathEl.nextElementSibling;
      if (labelEl && labelEl.classList.contains('edge-label')) {
        labelEl.setAttribute('x', (sp.x + tp.x) / 2);
        labelEl.setAttribute('y', midY);
      }

      var dot1 = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
      dot1.setAttribute('cx', sp.x);
      dot1.setAttribute('cy', sp.y);
      dot1.setAttribute('r', '5');
      dot1.setAttribute('class', 'conn-dot conn-dot-src');
      g.appendChild(dot1);

      var dot2 = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
      dot2.setAttribute('cx', tp.x);
      dot2.setAttribute('cy', tp.y);
      dot2.setAttribute('r', '3');
      dot2.setAttribute('class', 'conn-dot conn-dot-tgt');
      g.appendChild(dot2);
    });
  }

  // ── Draggable nodes ───────────────────────────────────────────────────────────

  var dragNode    = null;
  var dragStartX  = 0, dragStartY  = 0;
  var dragOffsetX = 0, dragOffsetY = 0;
  var dragMoved   = false;
  var DRAG_THRESHOLD = 4;

  function enableNodeDrag() {
    if (!svgCanvas) return;

    svgCanvas.addEventListener('mousedown', function (e) {
      var nodeGroup = e.target.closest('.graph-node');
      if (!nodeGroup) return;
      e.stopPropagation();
      dragNode  = nodeGroup;
      dragMoved = false;

      var pt = svgCanvas.createSVGPoint();
      pt.x = e.clientX; pt.y = e.clientY;
      var gEl = svgCanvas.querySelector('g');
      if (!gEl) return;
      var ctm = gEl.getScreenCTM();
      if (!ctm) return;
      var svgPt = pt.matrixTransform(ctm.inverse());
      dragStartX  = svgPt.x;
      dragStartY  = svgPt.y;
      dragOffsetX = 0;
      dragOffsetY = 0;

      nodeGroup.style.opacity = '0.8';
    }, true);

    document.addEventListener('mousemove', function (e) {
      if (!dragNode || !svgCanvas) return;

      var pt = svgCanvas.createSVGPoint();
      pt.x = e.clientX; pt.y = e.clientY;
      var gEl = svgCanvas.querySelector('g');
      if (!gEl) return;
      var ctm = gEl.getScreenCTM();
      if (!ctm) return;
      var svgPt = pt.matrixTransform(ctm.inverse());

      dragOffsetX = svgPt.x - dragStartX;
      dragOffsetY = svgPt.y - dragStartY;

      if (!dragMoved && (Math.abs(dragOffsetX) > DRAG_THRESHOLD || Math.abs(dragOffsetY) > DRAG_THRESHOLD)) {
        dragMoved = true;
      }

      dragNode.setAttribute('transform', 'translate(' + dragOffsetX + ',' + dragOffsetY + ')');

      if (dragNode.dataset.children) {
        dragNode.dataset.children.split(',').forEach(function (cid) {
          var childEl = qs('[data-node-id="' + cid + '"]');
          if (childEl && childEl !== dragNode) {
            childEl.setAttribute('transform', 'translate(' + dragOffsetX + ',' + dragOffsetY + ')');
            if (nodePositions[cid]) {
              var co = nodePositions[cid];
              var cb = {
                cx: co._baseCx !== undefined ? co._baseCx : co.cx,
                cy: co._baseCy !== undefined ? co._baseCy : co.cy,
                w: co.w, h: co.h
              };
              nodePositions[cid] = {
                cx: cb.cx + dragOffsetX, cy: cb.cy + dragOffsetY,
                w: cb.w, h: cb.h,
                _baseCx: cb.cx, _baseCy: cb.cy
              };
            }
          }
        });
      }

      var nodeId = dragNode.dataset.nodeId;
      var r      = dragNode.querySelector('.node-rect');
      if (r && nodePositions[nodeId]) {
        var orig = nodePositions[nodeId];
        var base = {
          cx: orig._baseCx !== undefined ? orig._baseCx : orig.cx,
          cy: orig._baseCy !== undefined ? orig._baseCy : orig.cy,
          w: orig.w, h: orig.h
        };
        nodePositions[nodeId] = {
          cx: base.cx + dragOffsetX, cy: base.cy + dragOffsetY,
          w: base.w, h: base.h,
          _baseCx: base.cx, _baseCy: base.cy
        };
      }
      updateEdges();
    });

    document.addEventListener('mouseup', function () {
      if (!dragNode) return;

      var r = dragNode.querySelector('.node-rect');
      if (r) {
        var w  = parseFloat(r.getAttribute('width'));
        var h  = parseFloat(r.getAttribute('height'));
        var ox = parseFloat(r.getAttribute('x'));
        var oy = parseFloat(r.getAttribute('y'));
        var newCx = snapToGrid(ox + w / 2 + dragOffsetX);
        var newCy = snapToGrid(oy + h / 2 + dragOffsetY);

        dragNode.querySelectorAll('*').forEach(function (el) { el.style.transition = 'none'; });

        r.setAttribute('x', newCx - w / 2);
        r.setAttribute('y', newCy - h / 2);

        var isBoundary = dragNode.dataset.nodeType === 'boundary';
        dragNode.querySelectorAll('text').forEach(function (t) {
          if (t.classList.contains('node-label')) {
            if (isBoundary) {
              t.setAttribute('x', newCx - w / 2 + 16);
              t.setAttribute('y', newCy - h / 2 + 20);
            } else {
              t.setAttribute('x', newCx);
              t.setAttribute('y', newCy - 4);
            }
          } else if (t.classList.contains('node-badge') && !t.hasAttribute('font-size')) {
            t.setAttribute('x', newCx);
            t.setAttribute('y', newCy + 14);
          }
        });

        dragNode.querySelectorAll('circle').forEach(function (c) {
          if (c.classList.contains('entry-marker') || c.classList.contains('exit-marker')) {
            c.setAttribute('cx', newCx - w / 2 + 10);
            c.setAttribute('cy', newCy - h / 2 + 10);
          }
        });
        dragNode.querySelectorAll('text[font-size=\'9\']').forEach(function (t) {
          t.setAttribute('x', newCx - w / 2 + 20);
          t.setAttribute('y', newCy - h / 2 + 14);
        });

        dragNode.removeAttribute('transform');

        var nodeGroupRef = dragNode;
        requestAnimationFrame(function () {
          nodeGroupRef.querySelectorAll('*').forEach(function (el) { el.style.transition = ''; });
        });

        var nodeId = dragNode.dataset.nodeId;
        nodePositions[nodeId] = { cx: newCx, cy: newCy, w: w, h: h };

        if (dragNode.dataset.children) {
          dragNode.dataset.children.split(',').forEach(function (cid) {
            var childEl = qs('[data-node-id="' + cid + '"]');
            if (!childEl || childEl === dragNode) return;
            var cr  = childEl.querySelector('.node-rect');
            if (!cr) return;
            var cw  = parseFloat(cr.getAttribute('width'));
            var ch  = parseFloat(cr.getAttribute('height'));
            var cox = parseFloat(cr.getAttribute('x'));
            var coy = parseFloat(cr.getAttribute('y'));
            var cnx = snapToGrid(cox + cw / 2 + dragOffsetX);
            var cny = snapToGrid(coy + ch / 2 + dragOffsetY);

            childEl.querySelectorAll('*').forEach(function (el) { el.style.transition = 'none'; });
            cr.setAttribute('x', cnx - cw / 2);
            cr.setAttribute('y', cny - ch / 2);

            var cTexts       = childEl.querySelectorAll('text');
            var childHasBadge = Array.prototype.some.call(cTexts, function (t) {
              return t.classList.contains('node-badge');
            });
            cTexts.forEach(function (t) {
              if (t.classList.contains('node-label')) {
                t.setAttribute('x', cnx);
                t.setAttribute('y', childHasBadge ? cny - 4 : cny + 4);
              } else if (t.classList.contains('node-badge') && !t.hasAttribute('font-size')) {
                t.setAttribute('x', cnx);
                t.setAttribute('y', cny + 14);
              }
            });
            childEl.removeAttribute('transform');
            nodePositions[cid] = { cx: cnx, cy: cny, w: cw, h: ch };
            requestAnimationFrame(function () {
              childEl.querySelectorAll('*').forEach(function (el) { el.style.transition = ''; });
            });
          });
        }
      }

      updateEdges();
      dragNode.style.opacity = '';

      if (dragMoved) {
        svgCanvas.addEventListener('click', function suppressClick(e) {
          e.stopPropagation();
          svgCanvas.removeEventListener('click', suppressClick, true);
        }, true);
      }

      dragNode   = null;
      dragMoved  = false;
    });
  }

  enableNodeDrag();

  // ── Viewport actions ──────────────────────────────────────────────────────────

  function fitContents() {
    svgScale = 1; svgPanX = 0; svgPanY = 0;
    updateSvgTransform();
  }

  // ── Filter functions ──────────────────────────────────────────────────────────

  function toggleFilterPopup() {
    var existing = qs('.filter-popup');
    if (existing) { existing.remove(); filterPopupVisible = false; return; }
    filterPopupVisible = true;

    var types = {};
    qsa('.graph-node').forEach(function (el) {
      var t = el.dataset.nodeType;
      if (t) types[t] = true;
    });

    var container = qs('.graph-canvas-container');
    if (!container) return;

    var popup = document.createElement('div');
    popup.className = 'filter-popup';

    var title = document.createElement('div');
    title.className   = 'filter-popup-title';
    title.textContent = 'Filter by type';
    popup.appendChild(title);

    Object.keys(types).sort().forEach(function (t) {
      var row = document.createElement('label');
      row.className   = 'filter-row';

      var cb       = document.createElement('input');
      cb.type      = 'checkbox';
      cb.checked   = !activeFilters[t];
      cb.addEventListener('change', function () {
        if (cb.checked) delete activeFilters[t];
        else activeFilters[t] = true;
        applyFilters();
      });

      var dot = document.createElement('span');
      dot.className  = 'filter-dot';
      dot.style.background = TYPE_COLORS[t] || '#8b949e';

      var lbl = document.createElement('span');
      lbl.className   = 'filter-type-label';
      lbl.textContent = t;

      row.appendChild(cb);
      row.appendChild(dot);
      row.appendChild(lbl);
      popup.appendChild(row);
    });

    container.appendChild(popup);
  }

  function applyFilters() {
    var hasFilters = Object.keys(activeFilters).length > 0;
    if (!hasFilters) {
      qsa('.node-filtered').forEach(function (el) { el.classList.remove('node-filtered'); });
      qsa('.edge-filtered').forEach(function (el) { el.classList.remove('edge-filtered'); });
      return;
    }

    qsa('.graph-node').forEach(function (el) {
      var t = el.dataset.nodeType;
      if (t && activeFilters[t]) el.classList.add('node-filtered');
      else el.classList.remove('node-filtered');
    });

    var filteredIds = {};
    qsa('.graph-node.node-filtered').forEach(function (el) {
      filteredIds[el.dataset.nodeId] = true;
    });

    qsa('.edge-path').forEach(function (edgeEl) {
      var src = edgeEl.dataset.edgeSrc;
      var tgt = edgeEl.dataset.edgeTgt;
      if (!src || !tgt) return;
      if (filteredIds[src] || filteredIds[tgt]) edgeEl.classList.add('edge-filtered');
      else edgeEl.classList.remove('edge-filtered');
    });

    qsa('.edge-label').forEach(function (labelEl) {
      var src = labelEl.dataset.edgeSrc;
      var tgt = labelEl.dataset.edgeTgt;
      if (!src || !tgt) return;
      if (filteredIds[src] || filteredIds[tgt]) labelEl.classList.add('edge-filtered');
      else labelEl.classList.remove('edge-filtered');
    });
  }

  // ── Layout toolbar ────────────────────────────────────────────────────────────

  var currentLayout = 'hierarchical';

  function addLayoutToolbar() {
    var container = qs('.graph-canvas-container');
    if (!container) return;

    var toolbar = document.createElement('div');
    toolbar.className = 'layout-toolbar';

    var layouts = [
      {
        id:    'hierarchical',
        title: 'Hierarchical (top-down)',
        icon:  '<svg width="20" height="20" viewBox="0 0 20 20" fill="currentColor"><rect x="7" y="1" width="6" height="4" rx="1"/><rect x="1" y="9" width="6" height="4" rx="1"/><rect x="13" y="9" width="6" height="4" rx="1"/><line x1="10" y1="5" x2="4" y2="9" stroke="currentColor" stroke-width="1.2"/><line x1="10" y1="5" x2="16" y2="9" stroke="currentColor" stroke-width="1.2"/></svg>'
      },
      {
        id:    'horizontal',
        title: 'Horizontal (left-right)',
        icon:  '<svg width="20" height="20" viewBox="0 0 20 20" fill="currentColor"><rect x="1" y="7" width="4" height="6" rx="1"/><rect x="9" y="1" width="4" height="6" rx="1"/><rect x="9" y="13" width="4" height="6" rx="1"/><line x1="5" y1="10" x2="9" y2="4" stroke="currentColor" stroke-width="1.2"/><line x1="5" y1="10" x2="9" y2="16" stroke="currentColor" stroke-width="1.2"/></svg>'
      }
    ];

    layouts.forEach(function (l) {
      var btn = document.createElement('button');
      btn.className        = 'layout-btn' + (l.id === currentLayout ? ' active' : '');
      btn.innerHTML        = l.icon;
      btn.title            = l.title;
      btn.dataset.layout   = l.id;
      btn.addEventListener('click', function () {
        saveCurrentLayout();
        currentLayout = l.id;
        qsa('.layout-btn').forEach(function (b) { b.classList.remove('active'); });
        btn.classList.add('active');
        reloadCurrentView();
      });
      toolbar.appendChild(btn);
    });

    var sep1 = document.createElement('div');
    sep1.className = 'layout-toolbar-sep';
    toolbar.appendChild(sep1);

    var fitBtn = document.createElement('button');
    fitBtn.className = 'layout-btn';
    fitBtn.title     = 'Fit to screen';
    fitBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="2" width="20" height="20" rx="3"/><rect x="6" y="8" width="10" height="8" rx="1.5"/><polyline points="6 8 2 4"/><polyline points="16 8 20 4"/></svg>';
    fitBtn.addEventListener('click', fitContents);
    toolbar.appendChild(fitBtn);

    var sep2 = document.createElement('div');
    sep2.className = 'layout-toolbar-sep';
    toolbar.appendChild(sep2);

    var filterBtn = document.createElement('button');
    filterBtn.className = 'layout-btn filter-toggle-btn';
    filterBtn.title     = 'Filter by type';
    filterBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"/></svg>';
    filterBtn.addEventListener('click', function (e) {
      e.stopPropagation();
      toggleFilterPopup();
    });
    toolbar.appendChild(filterBtn);

    var sep3 = document.createElement('div');
    sep3.className = 'layout-toolbar-sep';
    toolbar.appendChild(sep3);

    var canvasBtn = document.createElement('button');
    canvasBtn.className = 'layout-btn canvas-theme-btn';
    canvasBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>';
    canvasBtn.title     = 'Light canvas';
    canvasBtn.addEventListener('click', toggleCanvasTheme);
    toolbar.appendChild(canvasBtn);

    container.appendChild(toolbar);
  }

  function reloadCurrentView() {
    navigateTo(currentLevel, currentModule, currentFunction, currentBlock, false);
  }

  addLayoutToolbar();

  // ── Resizable panels ──────────────────────────────────────────────────────────

  function dragResize(handle, target, axis, sign, min, max) {
    handle.addEventListener('mousedown', function (e) {
      e.preventDefault();
      var startPos  = axis === 'x' ? e.clientX : e.clientY;
      var startSize = axis === 'x'
        ? target.getBoundingClientRect().width
        : target.getBoundingClientRect().height;
      target.style.transition      = 'none';
      handle.classList.add('active');
      document.body.style.cursor    = axis === 'x' ? 'col-resize' : 'row-resize';
      document.body.style.userSelect = 'none';

      function onMove(e2) {
        var current = axis === 'x' ? e2.clientX : e2.clientY;
        var delta   = (current - startPos) * sign;
        var newSize = Math.max(0, startSize + delta);
        if (newSize < min) {
          target.style.display = 'none';
          if (axis === 'x') target.style.width  = '0px';
          else              target.style.height = '0px';
        } else {
          target.style.display = '';
          if (axis === 'x') target.style.width  = Math.min(newSize, max) + 'px';
          else              target.style.height = Math.min(newSize, max) + 'px';
        }
      }

      function onUp() {
        handle.classList.remove('active');
        target.style.transition      = '';
        document.body.style.cursor    = '';
        document.body.style.userSelect = '';
        document.removeEventListener('mousemove', onMove);
        document.removeEventListener('mouseup',   onUp);
      }

      document.addEventListener('mousemove', onMove);
      document.addEventListener('mouseup',   onUp);
    });
  }

  function insertResizeHandles() {
    var sidebar   = qs('.studio-sidebar');
    var inspector = qs('.studio-inspector');
    var chatPanel = qs('.chat-panel');

    if (sidebar && sidebar.nextElementSibling) {
      var h1 = document.createElement('div');
      h1.className = 'resize-handle resize-handle-h';
      sidebar.parentNode.insertBefore(h1, sidebar.nextSibling);
      dragResize(h1, sidebar, 'x', 1, 60, 500);
    }

    if (inspector) {
      var h2 = document.createElement('div');
      h2.className = 'resize-handle resize-handle-h';
      inspector.parentNode.insertBefore(h2, inspector);
      dragResize(h2, inspector, 'x', -1, 60, 500);
    }

    if (chatPanel) {
      var hv = document.createElement('div');
      hv.className = 'resize-handle resize-handle-v';
      chatPanel.parentNode.insertBefore(hv, chatPanel);
      dragResize(hv, chatPanel, 'y', -1, 50, 600);
    }
  }

  insertResizeHandles();

  // ── Panel toggle buttons ──────────────────────────────────────────────────────

  var panelIcons = {
    left:   '<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><rect x="1" y="1" width="14" height="14" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/><rect x="1" y="1" width="5" height="14" rx="1" fill="currentColor" opacity="0.4"/></svg>',
    bottom: '<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><rect x="1" y="1" width="14" height="14" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/><rect x="1" y="10" width="14" height="5" rx="1" fill="currentColor" opacity="0.4"/></svg>',
    right:  '<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><rect x="1" y="1" width="14" height="14" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/><rect x="10" y="1" width="5" height="14" rx="1" fill="currentColor" opacity="0.4"/></svg>'
  };

  function addPanelToggles() {
    var headerRight = qs('.header-right');
    if (!headerRight) return;

    var togglesDiv = document.createElement('div');
    togglesDiv.className = 'panel-toggles';

    var panels = [
      { sel: '.studio-sidebar',   icon: panelIcons.left,   title: 'Explorer (left panel)'    },
      { sel: '.chat-panel',       icon: panelIcons.bottom, title: 'Chat (bottom panel)'       },
      { sel: '.studio-inspector', icon: panelIcons.right,  title: 'Inspector (right panel)'   }
    ];

    panels.forEach(function (p) {
      var btn = document.createElement('button');
      btn.className = 'panel-toggle-btn active';
      btn.innerHTML = p.icon;
      btn.title     = p.title;
      btn.addEventListener('click', function () {
        var el = qs(p.sel);
        if (!el) return;
        if (el.style.display === 'none') {
          el.style.display = '';
          btn.classList.add('active');
        } else {
          el.style.display = 'none';
          btn.classList.remove('active');
        }
      });
      togglesDiv.appendChild(btn);
    });

    var themeBtn2 = headerRight.querySelector('.theme-toggle');
    headerRight.insertBefore(togglesDiv, themeBtn2);
  }

  // addPanelToggles(); — disabled in Phase 15 (sidebar/chat/inspector toggles replaced by footer navigation)

  // ── Code view ─────────────────────────────────────────────────────────────────

  function extractBlockOps(parsed, functionName, blockLabel) {
    var functions = parsed['duumbi:functions'];
    if (!Array.isArray(functions)) return parsed;
    for (var i = 0; i < functions.length; i++) {
      var fn = functions[i];
      if (fn['duumbi:name'] !== functionName) continue;
      var blocks = fn['duumbi:blocks'];
      if (!Array.isArray(blocks)) return parsed;
      for (var j = 0; j < blocks.length; j++) {
        var block = blocks[j];
        if (block['duumbi:label'] === blockLabel) return block;
      }
    }
    return parsed;
  }

  function toggleCodeView() {
    if (codeViewActive) {
      codeViewActive = false;
      var codeView = qs('.code-view');
      if (codeView) codeView.remove();
      var svg = qs('.graph-canvas');
      if (svg) svg.style.display = '';
      var tabs    = qs('.c4-tabs');
      if (tabs)    tabs.style.display = '';
      var toolbar = qs('.layout-toolbar');
      if (toolbar) toolbar.style.display = '';
      if (lastGraphData) renderGraph(lastGraphData);
      updateBreadcrumb();
    } else {
      codeViewActive = true;
      updateBreadcrumb();
      var module = currentModule || 'app/main';
      fetch('/api/source?module=' + encodeURIComponent(module))
        .then(function (r) { return r.json(); })
        .then(function (data) {
          if (data.error) { console.error('Source API error:', data.error); return; }
          var displaySource = data.source;
          if (currentLevel === 'code' && currentFunction && currentBlock) {
            try {
              var parsed   = JSON.parse(data.source);
              var blockObj = extractBlockOps(parsed, currentFunction, currentBlock);
              displaySource = JSON.stringify(blockObj, null, 2);
            } catch (ex) {
              // fallback: show full source
            }
          }
          renderCodeView(displaySource);
        })
        .catch(function (ex) { console.error('Source fetch error:', ex); });
    }
  }

  function escapeHtml(str) {
    return str
      .replace(/&/g,  '&amp;')
      .replace(/</g,  '&lt;')
      .replace(/>/g,  '&gt;')
      .replace(/"/g,  '&quot;');
  }

  function renderCodeView(source) {
    var tabs    = qs('.c4-tabs');
    if (tabs)    tabs.style.display    = 'none';
    var toolbar = qs('.layout-toolbar');
    if (toolbar) toolbar.style.display = 'none';
    var svg = qs('.graph-canvas');
    if (svg) svg.style.display = 'none';

    var existing = qs('.code-view');
    if (existing) existing.remove();

    var pretty;
    try { pretty = JSON.stringify(JSON.parse(source), null, 2); }
    catch (ex) { pretty = source; }

    var lines       = pretty.split('\n');
    var html        = '';
    var foldTargets = {};

    for (var i = 0; i < lines.length; i++) {
      var trimmed  = lines[i].trimEnd();
      var lastChar = trimmed[trimmed.length - 1];
      if (lastChar === '{' || lastChar === '[') {
        var openChar  = lastChar;
        var closeChar = openChar === '{' ? '}' : ']';
        var depth     = 1;
        for (var j = i + 1; j < lines.length; j++) {
          var lt = lines[j].trimStart();
          for (var c = 0; c < lt.length; c++) {
            if (lt[c] === openChar)  depth++;
            else if (lt[c] === closeChar) {
              depth--;
              if (depth === 0) { if (j > i + 1) foldTargets[i] = j; break; }
            }
          }
          if (depth === 0) break;
        }
      }
    }

    for (var i = 0; i < lines.length; i++) {
      var hasFold  = foldTargets.hasOwnProperty(i);
      var foldAttr = hasFold
        ? ' data-fold-start="' + i + '" data-fold-end="' + foldTargets[i] + '"'
        : '';
      var foldMarker = hasFold
        ? '<span class="code-fold-marker" data-fold="' + i + '">&#9660;</span>'
        : '<span class="code-fold-marker-spacer"></span>';
      var highlighted = highlightJson(escapeHtml(lines[i]));
      html += '<div class="code-line" data-line="' + i + '"' + foldAttr + '>'
        + '<span class="line-number">' + (i + 1) + '</span>'
        + foldMarker
        + '<span class="line-content">' + highlighted + '</span>'
        + '</div>';
    }

    var container = qs('.graph-canvas-container');
    var codeDiv   = document.createElement('div');
    codeDiv.className = 'code-view';
    codeDiv.innerHTML = html;
    container.appendChild(codeDiv);

    codeDiv.addEventListener('click', function (e) {
      var marker = e.target.closest('.code-fold-marker');
      if (!marker) return;
      var foldIdx    = parseInt(marker.dataset.fold, 10);
      var endLine    = foldTargets[foldIdx];
      if (endLine === undefined) return;
      var isCollapsed = marker.classList.contains('collapsed');
      for (var k = foldIdx + 1; k < endLine; k++) {
        var lineEl = codeDiv.querySelector('.code-line[data-line="' + k + '"]');
        if (lineEl) lineEl.style.display = isCollapsed ? '' : 'none';
      }
      marker.classList.toggle('collapsed');
      marker.innerHTML = isCollapsed ? '&#9660;' : '&#9654;';
    });
  }

  function highlightJson(escaped) {
    escaped = escaped.replace(/:\s*(null|true|false)(?!\w)/g, ': <span class="code-keyword">$1</span>');
    escaped = escaped.replace(/:\s*(-?\d+\.?\d*)(\s*[,\r\n]?)/g, ': <span class="code-number">$1</span>$2');
    escaped = escaped.replace(/(&quot;)((?:@[\w:]+|[\w:@\-./]+))(&quot;)\s*:/g,
      '<span class="code-key">$1$2$3</span>:');
    escaped = escaped.replace(/:\s*(&quot;)(.*?)(&quot;)/g,
      ': <span class="code-string">$1$2$3</span>');
    escaped = escaped.replace(/(&quot;)((?!<\/span>).*?)(&quot;)(?![^<]*<\/span>)/g,
      '<span class="code-string">$1$2$3</span>');
    return escaped;
  }

  // Reset code view when navigating away from code level
  var _origNavigateTo = navigateTo;
  navigateTo = function (level, module, func, block, restoreLayout) {
    if (codeViewActive && level !== 'code') {
      codeViewActive = false;
      var codeView = qs('.code-view');
      if (codeView) codeView.remove();
      var svg = qs('.graph-canvas');
      if (svg) svg.style.display = '';
      var tabs    = qs('.c4-tabs');
      if (tabs)    tabs.style.display = '';
      var toolbar = qs('.layout-toolbar');
      if (toolbar) toolbar.style.display = '';
    }
    _origNavigateTo(level, module, func, block, restoreLayout);
  };

  // ── Chat ──────────────────────────────────────────────────────────────────────

  var chatMode = 'query';

  function setChatMode(mode) {
    if (['query', 'agent', 'intent'].indexOf(mode) === -1) mode = 'query';
    chatMode = mode;
    qsa('.chat-mode-tab').forEach(function (btn) {
      btn.classList.toggle('active', btn.getAttribute('data-mode') === mode);
    });
    var input = qs('.chat-input') || document.getElementById('chatInput');
    if (input) {
      input.placeholder = mode === 'query'
        ? 'Ask about this graph...'
        : mode === 'agent'
          ? 'Describe a graph change...'
          : 'Describe an intent...';
    }
  }

  function sendChat() {
    var chatInput    = qs('.chat-input') || document.getElementById('chatInput');
    var chatMessages = qs('.chat-messages') || document.getElementById('chatMessages');
    if (!chatInput || !chatMessages) return;

    var msg = chatInput.value.trim();
    if (!msg) return;

    var userDiv = document.createElement('div');
    userDiv.className   = 'chat-msg user';
    userDiv.textContent = msg;
    chatMessages.appendChild(userDiv);
    chatInput.value = '';
    chatMessages.scrollTop = chatMessages.scrollHeight;

    // Use WebSocket if connected, otherwise fallback placeholder
    if (StudioWS.isConnected()) {
      if (chatMode === 'query') {
        StudioWS.beginPending('Reviewer agent is answering');
      }
      StudioWS.send(msg, '', currentLevel, chatMode);
    } else {
      var aiDiv = document.createElement('div');
      aiDiv.className = 'chat-msg ai';
      aiDiv.innerHTML =
        "I'm analyzing your request in the context of the graph. " +
        "Connecting to the AI backend...<div class=\"msg-meta\">DUUMBI AI &middot; just now</div>";
      chatMessages.appendChild(aiDiv);
      chatMessages.scrollTop = chatMessages.scrollHeight;
    }
  }

  function handleChatKey(e) {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); sendChat(); }
  }

  // Wire up existing chat elements (SSR and design-shell IDs)
  var chatSendBtn = qs('.chat-send') || qs('[onclick="sendChat()"]');
  var chatInputEl = qs('.chat-input') || document.getElementById('chatInput');

  if (chatSendBtn) chatSendBtn.addEventListener('click', sendChat);
  if (chatInputEl) chatInputEl.addEventListener('keydown', handleChatKey);

  // ── WebSocket ─────────────────────────────────────────────────────────────────

  var StudioWS = (function () {
    var _ws          = null;
    var _port        = null;
    var _reconnTimer = null;

    // Event handler arrays
    var _onChunkCbs  = [];
    var _onResultCbs = [];
    var _onErrorCbs  = [];

    function _fire(cbs, payload) {
      cbs.forEach(function (cb) {
        try { cb(payload); } catch (ex) { console.error('StudioWS handler error', ex); }
      });
    }

    function _schedule_reconnect() {
      if (_reconnTimer) return;
      _reconnTimer = setTimeout(function () {
        _reconnTimer = null;
        if (_port !== null) connect(_port);
      }, 5000);
    }

    function connect(port) {
      _port = port;
      if (_ws && (_ws.readyState === WebSocket.CONNECTING || _ws.readyState === WebSocket.OPEN)) {
        return; // already connected or connecting
      }

      var url = 'ws://localhost:' + port + '/ws/chat';
      try {
        _ws = new WebSocket(url);
      } catch (ex) {
        console.warn('StudioWS: failed to open WebSocket to', url, ex);
        _schedule_reconnect();
        return;
      }

      _ws.addEventListener('open', function () {
        console.info('StudioWS: connected to', url);
        if (_reconnTimer) { clearTimeout(_reconnTimer); _reconnTimer = null; }
      });

      _ws.addEventListener('message', function (ev) {
        var data;
        try { data = JSON.parse(ev.data); } catch (ex) {
          // Raw text chunk (streaming)
          _fire(_onChunkCbs, ev.data);
          return;
        }

        if (data.type === 'chunk') {
          _fire(_onChunkCbs, data.text || data.content || '');
        } else if (data.type === 'result' || data.type === 'done') {
          _fire(_onResultCbs, data);
          _appendChatResult(data);
        } else if (data.type === 'answer') {
          _appendChatAnswer(data);
        } else if (data.type === 'error') {
          _fire(_onErrorCbs, data.message || 'Unknown error');
          _appendChatError(data.message || 'Unknown error');
        } else {
          // Unknown frame type — treat as chunk
          _fire(_onChunkCbs, JSON.stringify(data));
        }
      });

      _ws.addEventListener('close', function () {
        console.info('StudioWS: disconnected');
        _ws = null;
        _schedule_reconnect();
      });

      _ws.addEventListener('error', function (ev) {
        console.warn('StudioWS: connection error', ev);
        _fire(_onErrorCbs, 'WebSocket error');
      });
    }

    // Send a chat frame to the server
    function send(message, module, c4Level, mode) {
      if (!_ws || _ws.readyState !== WebSocket.OPEN) {
        console.warn('StudioWS: not connected — message dropped');
        return;
      }
      var frame = {
        type:     'chat',
        mode:     mode || 'query',
        message:  message,
        module:   module  || '',
        c4_level: c4Level || 'context'
      };
      _ws.send(JSON.stringify(frame));
    }

    function isConnected() {
      return !!_ws && _ws.readyState === WebSocket.OPEN;
    }

    function onChunk(cb)  { _onChunkCbs.push(cb);  }
    function onResult(cb) { _onResultCbs.push(cb); }
    function onError(cb)  { _onErrorCbs.push(cb);  }

    // ── Internal chat DOM helpers ─────────────────────────────────────────────

    // Track partial streaming chunk into the last AI bubble
    var _streamingDiv = null;

    function splitThinkingBlocks(text) {
      var remainder = text || '';
      var answer = '';
      var thinking = [];
      while (true) {
        var start = remainder.indexOf('<think>');
        if (start === -1) {
          answer += remainder;
          break;
        }
        var contentStart = start + '<think>'.length;
        var end = remainder.indexOf('</think>', contentStart);
        if (end === -1) {
          return { thinking: '', answer: (text || '').trim() };
        }
        answer += remainder.slice(0, start);
        var thought = remainder.slice(contentStart, end).trim();
        if (thought) thinking.push(thought);
        remainder = remainder.slice(end + '</think>'.length);
      }
      return {
        thinking: thinking.join('\n\n'),
        answer: answer.trim()
      };
    }

    function formatConfidence(value) {
      var text = String(value || 'unknown').toLowerCase();
      return text.charAt(0).toUpperCase() + text.slice(1);
    }

    function _ensureStreamingBubble(pendingText) {
      var chatMessages = qs('.chat-messages') || document.getElementById('chatMessages');
      if (!chatMessages) return null;
      if (!_streamingDiv) {
        _streamingDiv = document.createElement('div');
        _streamingDiv.className = 'chat-msg ai streaming';
        if (pendingText) {
          var pending = document.createElement('div');
          pending.className = 'answer-pending';
          pending.textContent = pendingText;
          var dots = document.createElement('span');
          dots.className = 'pending-dots';
          pending.appendChild(dots);
          _streamingDiv.appendChild(pending);
        }
        chatMessages.appendChild(_streamingDiv);
      }
      return _streamingDiv;
    }

    function beginPending(text) {
      var bubble = _ensureStreamingBubble(text);
      var chatMessages = qs('.chat-messages') || document.getElementById('chatMessages');
      if (bubble && chatMessages) chatMessages.scrollTop = chatMessages.scrollHeight;
    }

    // Register default chunk handler: append to streaming bubble
    onChunk(function (text) {
      var bubble = _ensureStreamingBubble();
      if (!bubble) return;
      var pending = bubble.querySelector('.answer-pending');
      if (pending) pending.remove();
      bubble.textContent += text;
      var chatMessages = qs('.chat-messages') || document.getElementById('chatMessages');
      if (chatMessages) chatMessages.scrollTop = chatMessages.scrollHeight;
    });

    function _appendChatResult(data) {
      if (_streamingDiv) {
        _streamingDiv.classList.remove('streaming');
        var meta = document.createElement('div');
        meta.className   = 'msg-meta';
        meta.textContent = 'DUUMBI AI \u00B7 just now';
        _streamingDiv.appendChild(meta);
        _streamingDiv = null;
      }
      // Live graph refresh after successful mutation (#479).
      if (data.refresh) {
        reloadCurrentGraph(data.changed_nodes || []);
      }
    }

    function _appendChatAnswer(data) {
      if (_streamingDiv) {
        _streamingDiv.classList.remove('streaming');
        var pending = _streamingDiv.querySelector('.answer-pending');
        if (pending) pending.remove();
        var rawText = _streamingDiv.textContent || '';
        var parsed = splitThinkingBlocks(rawText);
        _streamingDiv.textContent = '';
        if (parsed.thinking) {
          var thinking = document.createElement('div');
          thinking.className = 'thinking-block';
          var label = document.createElement('span');
          label.className = 'thinking-label';
          label.textContent = 'Thinking: ';
          var body = document.createElement('span');
          body.textContent = parsed.thinking;
          thinking.appendChild(label);
          thinking.appendChild(body);
          _streamingDiv.appendChild(thinking);
        }
        if (parsed.answer) {
          var answerBody = document.createElement('div');
          answerBody.className = 'answer-body';
          answerBody.textContent = parsed.answer;
          _streamingDiv.appendChild(answerBody);
        }
        var meta = document.createElement('div');
        meta.className = 'msg-meta';
        var sourceCount = (data.sources || []).length;
        meta.textContent = 'Sources: ' + sourceCount
          + ' \u00B7 Confidence: ' + formatConfidence(data.confidence)
          + ' \u00B7 Model: ' + (data.model || 'unknown');
        _streamingDiv.appendChild(meta);
        if (data.suggested_handoff && data.suggested_handoff.mode) {
          var handoff = document.createElement('div');
          handoff.className = 'msg-meta';
          handoff.textContent = 'Suggested mode: ' + data.suggested_handoff.mode;
          _streamingDiv.appendChild(handoff);
        }
        _streamingDiv = null;
      }
    }

    function _appendChatError(message) {
      var chatMessages = qs('.chat-messages') || document.getElementById('chatMessages');
      if (!chatMessages) return;
      // Close any open streaming bubble
      if (_streamingDiv) { _streamingDiv.classList.remove('streaming'); _streamingDiv = null; }
      var errDiv = document.createElement('div');
      errDiv.className   = 'chat-msg ai error';
      errDiv.textContent = 'Error: ' + message;
      chatMessages.appendChild(errDiv);
      chatMessages.scrollTop = chatMessages.scrollHeight;
    }

    return {
      connect:     connect,
      send:        send,
      beginPending: beginPending,
      isConnected: isConnected,
      onChunk:     onChunk,
      onResult:    onResult,
      onError:     onError
    };
  }());

  // Auto-connect if the port is embedded in the HTML (set by Leptos SSR)
  (function () {
    var portMeta = document.querySelector('meta[name="duumbi-ws-port"]');
    if (portMeta) {
      var port = parseInt(portMeta.getAttribute('content'), 10);
      if (!isNaN(port)) StudioWS.connect(port);
    } else {
      // Fallback: try to read port from window global (set by server template)
      var winPort = window.__duumbiPort;
      if (winPort) StudioWS.connect(winPort);
    }
  }());

  // ── Expose public API ─────────────────────────────────────────────────────────

  window.__studio = {
    nav:        function (level, module, func, block) { navigateTo(level, module, func, block); },
    toggleCode: function () { toggleCodeView(); },
    ws:         StudioWS,

    // Design-shell entry points (called by inline onclick= attributes in HTML)
    toggleFunction:      toggleFunction,
    toggleExplorer:      toggleExplorer,
    toggleSidebarHeader: toggleSidebarFromHeader,
    togglePin:           togglePin,
    closeSidebar:        closeSidebarFull,
    toggleIntent:        toggleIntent,
    selectIntent:        selectIntent,
    selectC4:            selectC4,
    openCreateIntent:    openCreateIntent,
    closeCreateIntent:   closeCreateIntent,
    validateCip:         validateCip,
    createNewIntent:     createNewIntent,
    executeIntent:       executeIntent,
    runBuild:            runBuild,
    runBinary:           runBinary,
    openSearch:          openSearch,
    closeSearch:         closeSearch,
    closeCmdIfOutside:   closeCmdIfOutside,
    filterCmd:           filterCmd,
    openPopup:           openPopup,
    closeAllPopups:      closeAllPopups,
    toggleUserMenu:      toggleUserMenu,
    sendChat:            sendChat,
    handleChatKey:       handleChatKey,
    setChatMode:         setChatMode,

    // Settings
    openSettings:        openSettings,
    closeSettings:       closeSettings,
    saveProviders:       saveProviders,
    openAgentTemplates:  openAgentTemplates,
    addProviderCard:     addProviderCard,
    removeProviderCard:  removeProviderCard,
    onProviderChange:    onProviderChange,
    onAuthChange:        onAuthChange,
    onRoleChange:        onRoleChange,
    checkEnvStatus:      checkEnvStatus
  };

  // ── Initial load ──────────────────────────────────────────────────────────────

  navigateTo('context');

}());
