// DUUMBI Studio — Client-side interactivity for SSR mode.
// Adds click handlers, navigation, theme toggle, and chat functionality
// without requiring WASM hydration.

(function() {
  "use strict";

  // --- State ---
  let currentLevel = "context";
  let currentModule = null;
  let currentFunction = null;
  let currentBlock = null;

  // --- Helpers ---
  function qs(sel, ctx) { return (ctx || document).querySelector(sel); }
  function qsa(sel, ctx) { return (ctx || document).querySelectorAll(sel); }

  // --- Global theme toggle (affects the entire Studio UI) ---
  var globalTheme = "dark";
  var themeBtn = qs(".theme-toggle");
  if (themeBtn) {
    themeBtn.addEventListener("click", function() {
      if (globalTheme === "dark") {
        globalTheme = "light";
        document.body.classList.remove("theme-dark");
        document.body.classList.add("theme-light");
        themeBtn.textContent = "\u{1F319}";
        themeBtn.title = "Dark mode";
      } else {
        globalTheme = "dark";
        document.body.classList.remove("theme-light");
        document.body.classList.add("theme-dark");
        themeBtn.textContent = "\u2600";
        themeBtn.title = "Light mode";
      }
      // Canvas follows global theme unless explicitly overridden
      applyCanvasTheme();
    });
  }

  // --- Canvas theme toggle (only affects C4 graph area) ---
  // "auto" means follow global, "light"/"dark" means explicit override.
  var canvasTheme = "auto";

  function applyCanvasTheme() {
    var graphContainer = qs(".graph-canvas-container");
    if (!graphContainer) return;
    var resolved = canvasTheme === "auto" ? globalTheme : canvasTheme;
    graphContainer.classList.remove("canvas-light", "canvas-dark");
    if (resolved === "light") {
      graphContainer.classList.add("canvas-light");
    } else {
      graphContainer.classList.add("canvas-dark");
    }
    updateCanvasThemeBtn();
  }

  function toggleCanvasTheme() {
    // Cycle: auto → opposite of global → auto
    var opposite = globalTheme === "dark" ? "light" : "dark";
    canvasTheme = canvasTheme === "auto" ? opposite : "auto";
    applyCanvasTheme();
  }

  function updateCanvasThemeBtn() {
    var btn = qs(".canvas-theme-btn");
    if (!btn) return;
    var resolved = canvasTheme === "auto" ? globalTheme : canvasTheme;
    if (resolved === "dark") {
      btn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>';
      btn.title = "Light canvas";
    } else {
      btn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>';
      btn.title = "Dark canvas";
    }
  }

  // --- Graph rendering ---
  function renderGraph(data) {
    const svg = qs(".graph-canvas");
    if (!svg) return;

    const g = svg.querySelector("g");
    if (!g) return;

    // Clear existing content (keep defs)
    g.innerHTML = "";

    // Re-add dot grid background (cleared with g.innerHTML above)
    ensureDotGridPattern();
    addDotGridBackground();

    // Update viewBox
    if (data.bbox) {
      const b = data.bbox;
      const pad = 40;
      svg.setAttribute("viewBox",
        (b.min_x - pad) + " " + (b.min_y - pad) + " " +
        (b.max_x - b.min_x + 2*pad) + " " + (b.max_y - b.min_y + 2*pad));
    }

    // Render edges first (paths + labels; connection dots added after nodes)
    if (data.edges) {
      data.edges.forEach(function(edge) {
        // Skip self-loop edges (e.g. recursive calls)
        if (edge.source === edge.target) return;

        const pathEl = document.createElementNS("http://www.w3.org/2000/svg", "path");
        pathEl.setAttribute("d", edge.path_data);
        pathEl.setAttribute("class", "edge-path edge-" + (edge.edge_type || "default"));
        pathEl.setAttribute("marker-end", "url(#arrowhead)");
        pathEl.dataset.edgeSrc = edge.source;
        pathEl.dataset.edgeTgt = edge.target;
        g.appendChild(pathEl);

        if (edge.label && edge.label_x) {
          const text = document.createElementNS("http://www.w3.org/2000/svg", "text");
          text.setAttribute("x", edge.label_x);
          text.setAttribute("y", edge.label_y - 4);
          text.setAttribute("text-anchor", "middle");
          text.setAttribute("class", "edge-label");
          text.textContent = edge.label;
          g.appendChild(text);
        }
      });
    }

    // Render nodes
    if (data.nodes) {
      data.nodes.forEach(function(node) {
        const group = document.createElementNS("http://www.w3.org/2000/svg", "g");
        group.setAttribute("class", "graph-node node-" + (node.node_type || "default"));
        group.style.cursor = "pointer";
        group.dataset.nodeId = node.id;
        group.dataset.nodeType = node.node_type;
        // Mark entry/exit nodes
        if (node.node_type && node.node_type.indexOf("entry") !== -1) {
          group.dataset.entry = "true";
        }
        if (node.node_type && node.node_type.indexOf("exit") !== -1) {
          group.dataset.exit = "true";
        }

        // Snap initial position to the dot grid
        var nx = snapToGrid(node.x);
        var ny = snapToGrid(node.y);

        const rect = document.createElementNS("http://www.w3.org/2000/svg", "rect");
        rect.setAttribute("x", nx - node.width / 2);
        rect.setAttribute("y", ny - node.height / 2);
        rect.setAttribute("width", node.width);
        rect.setAttribute("height", node.height);
        rect.setAttribute("rx", "8");
        rect.setAttribute("ry", "8");
        rect.setAttribute("class", "node-rect");
        group.appendChild(rect);

        const label = document.createElementNS("http://www.w3.org/2000/svg", "text");
        label.setAttribute("x", nx);
        label.setAttribute("y", ny + 4);
        label.setAttribute("text-anchor", "middle");
        label.setAttribute("class", "node-label");
        label.textContent = node.label;
        group.appendChild(label);

        if (node.badge) {
          var badge = document.createElementNS("http://www.w3.org/2000/svg", "text");
          badge.setAttribute("x", nx);
          badge.setAttribute("y", ny + node.height / 2 - 6);
          badge.setAttribute("text-anchor", "middle");
          badge.setAttribute("class", "node-badge");
          badge.textContent = node.badge;
          group.appendChild(badge);
        }

        // Entry/exit markers: small colored indicator
        if (node.node_type && node.node_type.indexOf("entry") !== -1) {
          var marker = document.createElementNS("http://www.w3.org/2000/svg", "circle");
          marker.setAttribute("cx", nx - node.width / 2 + 10);
          marker.setAttribute("cy", ny - node.height / 2 + 10);
          marker.setAttribute("r", "5");
          marker.setAttribute("fill", "#3fb950");
          marker.setAttribute("class", "entry-marker");
          group.appendChild(marker);
          var mt = document.createElementNS("http://www.w3.org/2000/svg", "text");
          mt.setAttribute("x", nx - node.width / 2 + 20);
          mt.setAttribute("y", ny - node.height / 2 + 14);
          mt.setAttribute("class", "node-badge");
          mt.setAttribute("fill", "#3fb950");
          mt.setAttribute("font-size", "9");
          mt.textContent = "IN";
          group.appendChild(mt);
        }
        if (node.node_type && node.node_type.indexOf("exit") !== -1) {
          var emarker = document.createElementNS("http://www.w3.org/2000/svg", "circle");
          emarker.setAttribute("cx", nx - node.width / 2 + 10);
          emarker.setAttribute("cy", ny - node.height / 2 + 10);
          emarker.setAttribute("r", "5");
          emarker.setAttribute("fill", "#d29922");
          emarker.setAttribute("class", "exit-marker");
          group.appendChild(emarker);
          var et = document.createElementNS("http://www.w3.org/2000/svg", "text");
          et.setAttribute("x", nx - node.width / 2 + 20);
          et.setAttribute("y", ny - node.height / 2 + 14);
          et.setAttribute("class", "node-badge");
          et.setAttribute("fill", "#d29922");
          et.setAttribute("font-size", "9");
          et.textContent = "OUT";
          group.appendChild(et);
        }

        // Click handler — drill down
        group.addEventListener("click", function() {
          onNodeClick(node);
        });

        // Double-click — drill down deeper
        group.addEventListener("dblclick", function() {
          onNodeDblClick(node);
        });

        g.appendChild(group);
      });
    }

    // Store positions and compute proper edge routing with connection dots
    storeNodePositions();
    updateEdges();

    // Update module list from context data and cache for sidebar tree
    if (data.modules) {
      sidebarModules = data.modules;
    }
    // Cache functions/blocks from current data for sidebar expansion
    if (currentLevel === "container" && data.nodes) {
      sidebarFunctions[currentModule] = data.nodes.map(function(n) { return { id: n.id, label: n.label }; });
    }
    if (currentLevel === "component" && data.nodes) {
      var funcKey = currentModule + "/" + currentFunction;
      sidebarBlocks[funcKey] = data.nodes.map(function(n) { return { id: n.id, label: n.label }; });
    }

    updateSidebarTree();
  }

  // --- Update Inspector panel ---
  function updateInspector(node) {
    const inspector = qs(".inspector-panel");
    if (!inspector) return;

    inspector.innerHTML =
      '<h3 class="panel-title">Inspector</h3>' +
      '<div class="inspector-content">' +
        '<div class="inspector-field"><span class="field-label">@id</span><span class="field-value">' + node.id + '</span></div>' +
        '<div class="inspector-field"><span class="field-label">@type</span><span class="field-value">' + node.node_type + '</span></div>' +
        '<div class="inspector-field"><span class="field-label">label</span><span class="field-value">' + node.label + '</span></div>' +
        (node.badge ? '<div class="inspector-field"><span class="field-label">info</span><span class="field-value">' + node.badge + '</span></div>' : '') +
      '</div>';

    // Highlight selected node
    qsa(".graph-node").forEach(function(g) { g.classList.remove("selected"); });
    var target = qs('[data-node-id="' + node.id + '"]');
    if (target) target.classList.add("selected");
  }

  // --- Node double-click → Drill down ---
  function onNodeDblClick(node) {
    if (currentLevel === "context" && node.node_type === "module") {
      navigateTo("container", node.id);
    } else if (currentLevel === "container" && node.node_type === "function") {
      // Function id may include params in label, use just the name part
      var funcName = node.id;
      navigateTo("component", currentModule, funcName);
    } else if (currentLevel === "component" && node.node_type === "block") {
      navigateTo("code", currentModule, currentFunction, node.id);
    }
  }

  // --- Single click — inspector only, no navigation ---
  function onNodeClick(node) {
    updateInspector(node);
  }

  // --- Layout persistence across navigation ---
  // Stores per-view node positions and pan/zoom so manual arrangements survive
  // level switches. Key format: "context", "container/mod", "component/mod/fn", etc.
  var savedLayouts = {};

  function viewKey(level, mod, fn, blk) {
    var parts = [level || currentLevel];
    if (mod || currentModule) parts.push(mod || currentModule);
    if (fn  || currentFunction) parts.push(fn  || currentFunction);
    if (blk || currentBlock)    parts.push(blk || currentBlock);
    // Include layout type so different layouts have independent saved positions
    parts.push(currentLayout || "hierarchical");
    return parts.join("/");
  }

  function saveCurrentLayout() {
    var key = viewKey();
    savedLayouts[key] = {
      positions: JSON.parse(JSON.stringify(nodePositions)),
      panX: svgPanX,
      panY: svgPanY,
      scale: svgScale
    };
  }

  function applyLayoutIfSaved(key) {
    var saved = savedLayouts[key];
    if (!saved) return false;

    // Restore pan/zoom
    svgPanX = saved.panX;
    svgPanY = saved.panY;
    svgScale = saved.scale;
    updateSvgTransform();

    // Restore node positions
    qsa(".graph-node").forEach(function(group) {
      var nodeId = group.dataset.nodeId;
      var pos = saved.positions[nodeId];
      if (!pos) return;
      var r = group.querySelector(".node-rect");
      if (!r) return;
      var w = pos.w, h = pos.h, cx = pos.cx, cy = pos.cy;

      // Disable transitions for instant restore
      group.querySelectorAll("*").forEach(function(el) { el.style.transition = "none"; });

      r.setAttribute("x", cx - w / 2);
      r.setAttribute("y", cy - h / 2);
      group.querySelectorAll("text").forEach(function(t) {
        if (t.classList.contains("node-label")) {
          t.setAttribute("x", cx); t.setAttribute("y", cy + 4);
        } else if (t.classList.contains("node-badge") && !t.hasAttribute("font-size")) {
          t.setAttribute("x", cx); t.setAttribute("y", cy + h / 2 - 6);
        }
      });
      group.querySelectorAll("circle").forEach(function(c) {
        if (c.classList.contains("entry-marker") || c.classList.contains("exit-marker")) {
          c.setAttribute("cx", cx - w / 2 + 10); c.setAttribute("cy", cy - h / 2 + 10);
        }
      });
      group.querySelectorAll("text[font-size='9']").forEach(function(t) {
        t.setAttribute("x", cx - w / 2 + 20); t.setAttribute("y", cy - h / 2 + 14);
      });
      nodePositions[nodeId] = { cx: cx, cy: cy, w: w, h: h };

      requestAnimationFrame(function() {
        group.querySelectorAll("*").forEach(function(el) { el.style.transition = ""; });
      });
    });

    updateEdges();
    return true;
  }

  // --- Sidebar tree state ---
  var sidebarModules = [];  // cached module names from context level
  var sidebarFunctions = {}; // moduleId → [{id, label}]
  var sidebarBlocks = {};    // "moduleId/funcId" → [{id, label}]

  function updateSidebarTree() {
    var tree = qs(".module-tree");
    if (!tree) return;
    tree.innerHTML = "";

    sidebarModules.forEach(function(mod) {
      var isActiveModule = (currentModule === mod);

      // Module item
      var li = document.createElement("li");
      li.className = "module-item" + (isActiveModule ? " tree-active" : "");
      var arrow = isActiveModule ? "\u25BE" : "\u25B8"; // ▾ or ▸
      li.innerHTML = '<span class="tree-arrow">' + arrow + '</span>' +
        '<span class="module-name">' + mod + '</span>';
      li.style.cursor = "pointer";
      li.addEventListener("click", function() {
        navigateTo("container", mod);
      });
      tree.appendChild(li);

      // Expanded: show functions if this module is active
      if (isActiveModule && sidebarFunctions[mod]) {
        sidebarFunctions[mod].forEach(function(fn) {
          var isActiveFunc = (currentFunction === fn.id);
          var fnLi = document.createElement("li");
          fnLi.className = "module-item tree-child" + (isActiveFunc ? " tree-active" : "");
          var fnArrow = isActiveFunc ? "\u25BE" : "\u25B8";
          fnLi.innerHTML = '<span class="tree-arrow">' + fnArrow + '</span>' +
            '<span class="module-name">' + fn.label + '</span>';
          fnLi.style.cursor = "pointer";
          fnLi.addEventListener("click", function() {
            navigateTo("component", mod, fn.id);
          });
          tree.appendChild(fnLi);

          // Expanded: show blocks if this function is active
          var funcKey = mod + "/" + fn.id;
          if (isActiveFunc && sidebarBlocks[funcKey]) {
            sidebarBlocks[funcKey].forEach(function(blk) {
              var isActiveBlock = (currentBlock === blk.id);
              var blkLi = document.createElement("li");
              blkLi.className = "module-item tree-child tree-child-2" + (isActiveBlock ? " tree-active" : "");
              blkLi.innerHTML = '<span class="tree-arrow">\u25B8</span>' +
                '<span class="module-name">' + blk.label + '</span>';
              blkLi.style.cursor = "pointer";
              blkLi.addEventListener("click", function() {
                navigateTo("code", mod, fn.id, blk.id);
              });
              tree.appendChild(blkLi);
            });
          }
        });
      }
    });
  }

  // --- Navigation ---
  // restoreLayout: true = apply saved positions (level navigation)
  //                false = always use fresh server data (layout type change)
  function navigateTo(level, module, func, block, restoreLayout) {
    if (restoreLayout !== false) {
      saveCurrentLayout();
    }

    currentLevel = level;
    currentModule = module || null;
    currentFunction = func || null;
    currentBlock = block || null;

    // Update C4 tabs — active state + disabled for unreachable levels
    qsa(".c4-tab").forEach(function(tab) {
      var tabLevel = tab.textContent.toLowerCase();
      tab.classList.toggle("active", tabLevel === level);
      var reachable = tabLevel === "context"
        || (tabLevel === "container" && !!(module || currentModule))
        || (tabLevel === "component" && !!(func || currentFunction))
        || (tabLevel === "code" && !!(block || currentBlock));
      tab.classList.toggle("disabled", !reachable);
      tab.disabled = !reachable;
    });

    // Update breadcrumb
    updateBreadcrumb();

    var targetKey = viewKey(level, module, func, block);

    // Restore pan/zoom from saved layout (only when doing level navigation)
    if (restoreLayout !== false) {
      var saved = savedLayouts[targetKey];
      if (saved) {
        svgPanX = saved.panX; svgPanY = saved.panY; svgScale = saved.scale;
      } else {
        svgScale = 1; svgPanX = 0; svgPanY = 0;
      }
    } else {
      svgScale = 1; svgPanX = 0; svgPanY = 0;
    }
    updateSvgTransform();

    // Fetch data
    var url = "/api/graph/" + level;
    var params = [];
    if (module) params.push("module=" + encodeURIComponent(module));
    if (func) params.push("function=" + encodeURIComponent(func));
    if (block) params.push("block=" + encodeURIComponent(block));
    if (currentLayout && currentLayout !== "hierarchical") params.push("layout=" + currentLayout);
    if (params.length) url += "?" + params.join("&");

    fetch(url)
      .then(function(r) { return r.json(); })
      .then(function(data) {
        if (data.error) { console.error("API error:", data.error); return; }
        renderGraph(data);
        // Apply saved positions only for level navigation (not layout type change)
        if (restoreLayout !== false) { applyLayoutIfSaved(targetKey); }
        storeNodePositions();
        updateEdges();
      })
      .catch(function(e) { console.error("Fetch error:", e); });
  }

  function updateBreadcrumb() {
    const nav = qs(".breadcrumb");
    if (!nav) return;

    let html = '<button class="breadcrumb-item" onclick="window.__studio.nav(\'context\')">workspace</button>';
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
    nav.innerHTML = html;
  }

  // --- C4 tab clicks ---
  qsa(".c4-tab").forEach(function(tab) {
    tab.addEventListener("click", function() {
      const level = tab.textContent.toLowerCase();
      if (level === "context") {
        navigateTo("context");
      } else if (level === "container" && currentModule) {
        navigateTo("container", currentModule);
      } else if (level === "component" && currentModule && currentFunction) {
        navigateTo("component", currentModule, currentFunction);
      } else if (level === "code" && currentModule && currentFunction && currentBlock) {
        navigateTo("code", currentModule, currentFunction, currentBlock);
      }
    });
  });

  // --- Search overlay ---
  const searchBtn = qs(".search-btn");
  const shortcutsBtn = qs(".shortcuts-btn");

  if (searchBtn) {
    searchBtn.addEventListener("click", function() {
      toggleOverlay("search");
    });
  }
  if (shortcutsBtn) {
    shortcutsBtn.addEventListener("click", function() {
      toggleOverlay("shortcuts");
    });
  }

  function toggleOverlay(type) {
    // Simple overlay toggle — the SSR renders the overlays as hidden
    // For now, just log; full implementation with dynamic overlays
    console.log("Toggle overlay:", type);
  }

  // --- Keyboard shortcuts ---
  document.addEventListener("keydown", function(e) {
    if (e.key === "Escape") {
      // Close overlays
    }
    if (e.key === "?" && !e.ctrlKey && !e.metaKey) {
      const active = document.activeElement;
      if (active && (active.tagName === "INPUT" || active.tagName === "TEXTAREA")) return;
      toggleOverlay("shortcuts");
    }
    if ((e.ctrlKey || e.metaKey) && e.key === "k") {
      e.preventDefault();
      toggleOverlay("search");
    }
  });

  // --- Chat ---
  const chatInput = qs(".chat-input");
  const chatSend = qs(".chat-send");
  const chatMessages = qs(".chat-messages");

  function sendChat() {
    if (!chatInput || !chatMessages) return;
    const msg = chatInput.value.trim();
    if (!msg) return;

    // Add user message
    const userDiv = document.createElement("div");
    userDiv.className = "chat-message user-message";
    userDiv.textContent = msg;
    chatMessages.appendChild(userDiv);
    chatInput.value = "";
    chatMessages.scrollTop = chatMessages.scrollHeight;

    // Add thinking indicator
    const thinkDiv = document.createElement("div");
    thinkDiv.className = "chat-message system-message";
    thinkDiv.textContent = "Thinking...";
    chatMessages.appendChild(thinkDiv);

    // Call chat API (placeholder — uses server fn endpoint)
    fetch("/api/graph/context")
      .then(function() {
        thinkDiv.textContent = "Chat requires WASM hydration or a dedicated chat API endpoint.";
      })
      .catch(function() {
        thinkDiv.textContent = "Chat unavailable in SSR-only mode.";
      });
  }

  if (chatSend) chatSend.addEventListener("click", sendChat);
  if (chatInput) chatInput.addEventListener("keydown", function(e) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendChat();
    }
  });

  // --- Grid size constant ---
  var GRID_BASE = 12; // dot grid spacing — doubled density vs. original 24px
  var SNAP_STEP = GRID_BASE; // snap exactly to each dot

  // Snap value to nearest snap point
  function snapToGrid(val) {
    return Math.round(val / SNAP_STEP) * SNAP_STEP;
  }

  // Inject SVG dot-grid pattern into <defs> if not already present.
  // The pattern lives in SVG coordinate space inside the <g> element,
  // so it automatically follows pan/zoom without any extra updates.
  function ensureDotGridPattern() {
    if (!svgCanvas) return;
    var defs = svgCanvas.querySelector("defs");
    if (!defs) return;
    if (defs.querySelector("#dot-grid-pattern")) return;
    var ns = "http://www.w3.org/2000/svg";
    var pattern = document.createElementNS(ns, "pattern");
    pattern.setAttribute("id", "dot-grid-pattern");
    pattern.setAttribute("width", GRID_BASE);
    pattern.setAttribute("height", GRID_BASE);
    pattern.setAttribute("patternUnits", "userSpaceOnUse");
    var dot = document.createElementNS(ns, "circle");
    dot.setAttribute("cx", GRID_BASE / 2);
    dot.setAttribute("cy", GRID_BASE / 2);
    dot.setAttribute("r", "0.8");
    dot.setAttribute("class", "dot-grid-dot");
    pattern.appendChild(dot);
    defs.appendChild(pattern);
  }

  // Add/refresh the background rect that renders the dot grid inside <g>.
  // Must be called after every renderGraph() call that clears g.innerHTML.
  function addDotGridBackground() {
    if (!svgCanvas) return;
    var g = svgCanvas.querySelector("g");
    if (!g) return;
    if (g.querySelector(".dot-grid-bg")) return;
    var ns = "http://www.w3.org/2000/svg";
    var rect = document.createElementNS(ns, "rect");
    rect.setAttribute("class", "dot-grid-bg");
    rect.setAttribute("x", "-50000");
    rect.setAttribute("y", "-50000");
    rect.setAttribute("width", "100000");
    rect.setAttribute("height", "100000");
    rect.setAttribute("fill", "url(#dot-grid-pattern)");
    rect.setAttribute("pointer-events", "none");
    g.insertBefore(rect, g.firstChild);
  }

  // No-op: grid alignment is now handled by the SVG pattern (no CSS needed)
  function updateGrid() {}

  // --- SVG Pan/Zoom ---
  var svgCanvas = qs(".graph-canvas");
  var svgScale = 1;
  var svgPanX = 0, svgPanY = 0;
  var svgDragging = false;
  var svgLastX = 0, svgLastY = 0;

  function updateSvgTransform() {
    if (!svgCanvas) return;
    var g = svgCanvas.querySelector("g");
    if (g) g.setAttribute("transform", "translate(" + svgPanX + "," + svgPanY + ") scale(" + svgScale + ")");
    updateGrid();
  }

  // Convert screen (client) coordinates to SVG viewBox coordinates
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
    svgCanvas.addEventListener("wheel", function(e) {
      e.preventDefault();

      // macOS trackpad: ctrlKey=true for pinch-to-zoom, false for two-finger scroll
      if (!e.ctrlKey) {
        // Two-finger scroll → pan
        var pt1 = screenToSvg(0, 0);
        var pt2 = screenToSvg(e.deltaX, e.deltaY);
        svgPanX -= (pt2.x - pt1.x);
        svgPanY -= (pt2.y - pt1.y);
        updateSvgTransform();
        return;
      }

      // Pinch-to-zoom or ctrl+scroll → zoom
      var factor = 1 + Math.min(Math.abs(e.deltaY), 50) * 0.01;
      var delta = e.deltaY > 0 ? 1 / factor : factor;
      var newScale = svgScale * delta;
      newScale = Math.max(0.2, Math.min(4, newScale));

      // Zoom toward mouse position using accurate SVG coordinate conversion
      var mx = screenToSvg(e.clientX, e.clientY);
      svgPanX = mx.x - (mx.x - svgPanX) * (newScale / svgScale);
      svgPanY = mx.y - (mx.y - svgPanY) * (newScale / svgScale);
      svgScale = newScale;
      updateSvgTransform();
    }, { passive: false });

    svgCanvas.addEventListener("mousedown", function(e) {
      if (e.target.closest(".graph-node")) return;
      svgDragging = true;
      svgLastX = e.clientX;
      svgLastY = e.clientY;
    });

    svgCanvas.addEventListener("mousemove", function(e) {
      if (!svgDragging) return;
      // Convert screen pixel delta to viewBox coordinate delta
      var pt1 = screenToSvg(svgLastX, svgLastY);
      var pt2 = screenToSvg(e.clientX, e.clientY);
      svgPanX += (pt2.x - pt1.x);
      svgPanY += (pt2.y - pt1.y);
      svgLastX = e.clientX;
      svgLastY = e.clientY;
      updateSvgTransform();
    });

    svgCanvas.addEventListener("mouseup", function() { svgDragging = false; });
    svgCanvas.addEventListener("mouseleave", function() { svgDragging = false; });
  }

  // --- Node position tracking (for edge updates) ---
  // Maps nodeId -> {cx, cy, w, h} — updated during render and drag.
  var nodePositions = {};

  function storeNodePositions() {
    nodePositions = {};
    qsa(".graph-node").forEach(function(g) {
      var r = g.querySelector(".node-rect");
      if (!r) return;
      var id = g.dataset.nodeId;
      var w = parseFloat(r.getAttribute("width"));
      var h = parseFloat(r.getAttribute("height"));
      nodePositions[id] = {
        cx: parseFloat(r.getAttribute("x")) + w / 2,
        cy: parseFloat(r.getAttribute("y")) + h / 2,
        w: w, h: h
      };
    });
  }

  // Determine which side of a node rect faces toward a target point.
  // Returns "top", "bottom", "left", or "right".
  function borderSide(nodePos, tx, ty) {
    var dx = tx - nodePos.cx, dy = ty - nodePos.cy;
    var hw = nodePos.w / 2, hh = nodePos.h / 2;
    if (dx === 0 && dy === 0) return "bottom";
    var absDx = Math.abs(dx), absDy = Math.abs(dy);
    if (absDx * hh > absDy * hw) {
      return dx > 0 ? "right" : "left";
    } else {
      return dy > 0 ? "bottom" : "top";
    }
  }

  // Compute an evenly-distributed connection point on a node's side.
  // `index` is the 0-based slot, `count` is the total edges on that side.
  function distributedBorderPoint(nodePos, side, index, count) {
    var cx = nodePos.cx, cy = nodePos.cy;
    var hw = nodePos.w / 2, hh = nodePos.h / 2;
    var frac = (index + 1) / (count + 1); // e.g. 1 edge → 1/2, 2 edges → 1/3, 2/3
    switch (side) {
      case "top":
        return { x: cx - hw + nodePos.w * frac, y: cy - hh };
      case "bottom":
        return { x: cx - hw + nodePos.w * frac, y: cy + hh };
      case "left":
        return { x: cx - hw, y: cy - hh + nodePos.h * frac };
      case "right":
        return { x: cx + hw, y: cy - hh + nodePos.h * frac };
      default:
        return { x: cx, y: cy + hh };
    }
  }

  // Recompute all edge paths and connection dots from current node positions.
  function updateEdges() {
    var g = svgCanvas ? svgCanvas.querySelector("g") : null;
    if (!g) return;

    // Remove old connection dots
    g.querySelectorAll(".conn-dot").forEach(function(d) { d.remove(); });

    // Collect all edge-path elements
    var edgePaths = Array.prototype.slice.call(g.querySelectorAll(".edge-path"));

    // First pass: count edges per (node, side) so we can distribute them.
    // Key: "nodeId:side", value: array of edge indices
    var portMap = {};
    var edgeInfo = []; // parallel to edgePaths

    edgePaths.forEach(function(pathEl, i) {
      var srcId = pathEl.dataset.edgeSrc;
      var tgtId = pathEl.dataset.edgeTgt;
      // Hide self-loop edges (e.g. recursive calls)
      if (srcId === tgtId) {
        pathEl.style.display = "none";
        edgeInfo.push(null);
        return;
      }
      var src = nodePositions[srcId];
      var tgt = nodePositions[tgtId];
      if (!src || !tgt) {
        edgeInfo.push(null);
        return;
      }

      var srcSide = borderSide(src, tgt.cx, tgt.cy);
      var tgtSide = borderSide(tgt, src.cx, src.cy);

      var srcKey = srcId + ":" + srcSide;
      var tgtKey = tgtId + ":" + tgtSide;
      if (!portMap[srcKey]) portMap[srcKey] = [];
      if (!portMap[tgtKey]) portMap[tgtKey] = [];
      portMap[srcKey].push(i);
      portMap[tgtKey].push(i);

      edgeInfo.push({ srcId: srcId, tgtId: tgtId, srcSide: srcSide, tgtSide: tgtSide, srcKey: srcKey, tgtKey: tgtKey });
    });

    // Second pass: compute positions and draw paths + dots
    edgePaths.forEach(function(pathEl, i) {
      var info = edgeInfo[i];
      if (!info) return;
      var src = nodePositions[info.srcId];
      var tgt = nodePositions[info.tgtId];

      var srcSlots = portMap[info.srcKey];
      var tgtSlots = portMap[info.tgtKey];
      var srcIdx = srcSlots.indexOf(i);
      var tgtIdx = tgtSlots.indexOf(i);

      var sp = distributedBorderPoint(src, info.srcSide, srcIdx, srcSlots.length);
      var tp = distributedBorderPoint(tgt, info.tgtSide, tgtIdx, tgtSlots.length);

      // Orthogonal path
      var midY = (sp.y + tp.y) / 2;
      var d = "M " + sp.x + " " + sp.y +
              " L " + sp.x + " " + midY +
              " L " + tp.x + " " + midY +
              " L " + tp.x + " " + tp.y;
      pathEl.setAttribute("d", d);

      // Update label position
      var labelEl = pathEl.nextElementSibling;
      if (labelEl && labelEl.classList.contains("edge-label")) {
        labelEl.setAttribute("x", (sp.x + tp.x) / 2);
        labelEl.setAttribute("y", midY - 4);
      }

      // Source dot (origin) — larger
      var dot1 = document.createElementNS("http://www.w3.org/2000/svg", "circle");
      dot1.setAttribute("cx", sp.x);
      dot1.setAttribute("cy", sp.y);
      dot1.setAttribute("r", "5");
      dot1.setAttribute("class", "conn-dot conn-dot-src");
      g.appendChild(dot1);

      // Target dot (destination)
      var dot2 = document.createElementNS("http://www.w3.org/2000/svg", "circle");
      dot2.setAttribute("cx", tp.x);
      dot2.setAttribute("cy", tp.y);
      dot2.setAttribute("r", "3");
      dot2.setAttribute("class", "conn-dot conn-dot-tgt");
      g.appendChild(dot2);
    });
  }

  // --- Draggable nodes (using group transform) ---
  var dragNode = null;
  var dragStartX = 0, dragStartY = 0;
  var dragOffsetX = 0, dragOffsetY = 0;
  var dragMoved = false;
  var DRAG_THRESHOLD = 4; // px in SVG coords before drag is considered "real"

  function enableNodeDrag() {
    if (!svgCanvas) return;

    svgCanvas.addEventListener("mousedown", function(e) {
      var nodeGroup = e.target.closest(".graph-node");
      if (!nodeGroup) return;
      e.stopPropagation();
      dragNode = nodeGroup;
      dragMoved = false;

      // Convert screen coords to SVG coords
      var pt = svgCanvas.createSVGPoint();
      pt.x = e.clientX;
      pt.y = e.clientY;
      var g = svgCanvas.querySelector("g");
      if (!g) return;
      var ctm = g.getScreenCTM();
      if (!ctm) return;
      var svgPt = pt.matrixTransform(ctm.inverse());
      dragStartX = svgPt.x;
      dragStartY = svgPt.y;
      dragOffsetX = 0;
      dragOffsetY = 0;

      nodeGroup.style.opacity = "0.8";
    }, true);

    document.addEventListener("mousemove", function(e) {
      if (!dragNode || !svgCanvas) return;

      var pt = svgCanvas.createSVGPoint();
      pt.x = e.clientX;
      pt.y = e.clientY;
      var g = svgCanvas.querySelector("g");
      if (!g) return;
      var ctm = g.getScreenCTM();
      if (!ctm) return;
      var svgPt = pt.matrixTransform(ctm.inverse());

      dragOffsetX = svgPt.x - dragStartX;
      dragOffsetY = svgPt.y - dragStartY;

      if (!dragMoved && (Math.abs(dragOffsetX) > DRAG_THRESHOLD || Math.abs(dragOffsetY) > DRAG_THRESHOLD)) {
        dragMoved = true;
      }

      // Move the entire group via transform — all children move together
      dragNode.setAttribute("transform", "translate(" + dragOffsetX + "," + dragOffsetY + ")");

      // Update tracked position for edge routing
      var nodeId = dragNode.dataset.nodeId;
      var r = dragNode.querySelector(".node-rect");
      if (r && nodePositions[nodeId]) {
        var orig = nodePositions[nodeId];
        var base = {
          cx: orig._baseCx !== undefined ? orig._baseCx : orig.cx,
          cy: orig._baseCy !== undefined ? orig._baseCy : orig.cy,
          w: orig.w, h: orig.h
        };
        nodePositions[nodeId] = {
          cx: base.cx + dragOffsetX,
          cy: base.cy + dragOffsetY,
          w: base.w, h: base.h,
          _baseCx: base.cx, _baseCy: base.cy
        };
      }
      updateEdges();
    });

    document.addEventListener("mouseup", function() {
      if (!dragNode) return;

      // Bake the transform into actual coordinates and snap to grid
      var r = dragNode.querySelector(".node-rect");
      if (r) {
        var w = parseFloat(r.getAttribute("width"));
        var h = parseFloat(r.getAttribute("height"));
        var ox = parseFloat(r.getAttribute("x"));
        var oy = parseFloat(r.getAttribute("y"));
        var newCx = snapToGrid(ox + w / 2 + dragOffsetX);
        var newCy = snapToGrid(oy + h / 2 + dragOffsetY);

        // Disable transitions on all children to prevent snap-back animation,
        // then update coordinates and remove transform atomically.
        dragNode.querySelectorAll("*").forEach(function(el) {
          el.style.transition = "none";
        });

        r.setAttribute("x", newCx - w / 2);
        r.setAttribute("y", newCy - h / 2);

        var texts = dragNode.querySelectorAll("text");
        texts.forEach(function(t) {
          if (t.classList.contains("node-label")) {
            t.setAttribute("x", newCx);
            t.setAttribute("y", newCy + 4);
          } else if (t.classList.contains("node-badge") && !t.hasAttribute("font-size")) {
            t.setAttribute("x", newCx);
            t.setAttribute("y", newCy + h / 2 - 6);
          }
        });

        // Entry/exit markers
        var markers = dragNode.querySelectorAll("circle");
        markers.forEach(function(c) {
          if (c.classList.contains("entry-marker") || c.classList.contains("exit-marker")) {
            c.setAttribute("cx", newCx - w / 2 + 10);
            c.setAttribute("cy", newCy - h / 2 + 10);
          }
        });
        var markerTexts = dragNode.querySelectorAll("text[font-size='9']");
        markerTexts.forEach(function(t) {
          t.setAttribute("x", newCx - w / 2 + 20);
          t.setAttribute("y", newCy - h / 2 + 14);
        });

        // Remove group transform now that attributes are already at final position
        dragNode.removeAttribute("transform");

        // Re-enable transitions after the browser has committed the new layout
        var nodeGroupRef = dragNode;
        requestAnimationFrame(function() {
          nodeGroupRef.querySelectorAll("*").forEach(function(el) {
            el.style.transition = "";
          });
        });

        // Update stored position
        var nodeId = dragNode.dataset.nodeId;
        nodePositions[nodeId] = { cx: newCx, cy: newCy, w: w, h: h };
      }

      updateEdges();
      dragNode.style.opacity = "";

      // If the node was dragged, suppress the next click event so it doesn't trigger navigation
      if (dragMoved) {
        svgCanvas.addEventListener("click", function suppressClick(e) {
          e.stopPropagation();
          svgCanvas.removeEventListener("click", suppressClick, true);
        }, true);
      }

      dragNode = null;
      dragMoved = false;
    });
  }

  enableNodeDrag();

  // --- Viewport actions ---

  // Fit all nodes into the current canvas viewport.
  function fitContents() {
    // Reset to initial state — the viewBox already fits the content perfectly.
    svgScale = 1;
    svgPanX = 0;
    svgPanY = 0;
    updateSvgTransform();
  }

  // --- Layout Toolbar (right side of graph area) ---
  var currentLayout = "hierarchical";

  function addLayoutToolbar() {
    var container = qs(".graph-canvas-container");
    if (!container) return;

    var toolbar = document.createElement("div");
    toolbar.className = "layout-toolbar";

    var layouts = [
      {
        id: "hierarchical",
        title: "Hierarchical (top-down)",
        icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="currentColor"><rect x="7" y="1" width="6" height="4" rx="1" /><rect x="1" y="9" width="6" height="4" rx="1" /><rect x="13" y="9" width="6" height="4" rx="1" /><line x1="10" y1="5" x2="4" y2="9" stroke="currentColor" stroke-width="1.2"/><line x1="10" y1="5" x2="16" y2="9" stroke="currentColor" stroke-width="1.2"/></svg>'
      },
      {
        id: "horizontal",
        title: "Horizontal (left-right)",
        icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="currentColor"><rect x="1" y="7" width="4" height="6" rx="1" /><rect x="9" y="1" width="4" height="6" rx="1" /><rect x="9" y="13" width="4" height="6" rx="1" /><line x1="5" y1="10" x2="9" y2="4" stroke="currentColor" stroke-width="1.2"/><line x1="5" y1="10" x2="9" y2="16" stroke="currentColor" stroke-width="1.2"/></svg>'
      },
      {
        id: "radial",
        title: "Radial",
        icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="currentColor"><circle cx="10" cy="10" r="3" /><circle cx="10" cy="2" r="2" /><circle cx="17" cy="14" r="2" /><circle cx="3" cy="14" r="2" /><line x1="10" y1="7" x2="10" y2="4" stroke="currentColor" stroke-width="1.2"/><line x1="12.5" y1="12" x2="15.5" y2="13" stroke="currentColor" stroke-width="1.2"/><line x1="7.5" y1="12" x2="4.5" y2="13" stroke="currentColor" stroke-width="1.2"/></svg>'
      }
    ];

    layouts.forEach(function(l) {
      var btn = document.createElement("button");
      btn.className = "layout-btn" + (l.id === currentLayout ? " active" : "");
      btn.innerHTML = l.icon;
      btn.title = l.title;
      btn.dataset.layout = l.id;
      btn.addEventListener("click", function() {
        // Save current arrangement under old layout key before switching
        saveCurrentLayout();
        currentLayout = l.id;
        qsa(".layout-btn").forEach(function(b) { b.classList.remove("active"); });
        btn.classList.add("active");
        // Re-fetch with new layout — navigateTo will use new key, no stale positions
        reloadCurrentView();
      });
      toolbar.appendChild(btn);
    });

    // Separator
    var sep = document.createElement("div");
    sep.className = "layout-toolbar-sep";
    toolbar.appendChild(sep);

    // Fit to screen button
    var fitBtn = document.createElement("button");
    fitBtn.className = "layout-btn";
    fitBtn.title = "Fit to screen";
    fitBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="2" width="20" height="20" rx="3"/><rect x="6" y="8" width="10" height="8" rx="1.5"/><polyline points="6 8 2 4"/><polyline points="16 8 20 4"/></svg>';
    fitBtn.addEventListener("click", fitContents);
    toolbar.appendChild(fitBtn);

    // Separator
    var sep2 = document.createElement("div");
    sep2.className = "layout-toolbar-sep";
    toolbar.appendChild(sep2);

    // Canvas-only theme toggle button
    var canvasBtn = document.createElement("button");
    canvasBtn.className = "layout-btn canvas-theme-btn";
    canvasBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>';
    canvasBtn.title = "Light canvas";
    canvasBtn.addEventListener("click", toggleCanvasTheme);
    toolbar.appendChild(canvasBtn);

    container.appendChild(toolbar);
  }

  function reloadCurrentView() {
    // Layout type changed — always use fresh server data, don't restore saved positions
    navigateTo(currentLevel, currentModule, currentFunction, currentBlock, false);
  }

  addLayoutToolbar();

  // --- Resizable Panels ---
  // Generic drag-resize helper. Disables CSS transition during drag for instant feedback.
  function dragResize(handle, target, axis, sign, min, max) {
    handle.addEventListener("mousedown", function(e) {
      e.preventDefault();
      var startPos = axis === "x" ? e.clientX : e.clientY;
      var startSize = axis === "x"
        ? target.getBoundingClientRect().width
        : target.getBoundingClientRect().height;
      // Disable CSS transition during drag
      target.style.transition = "none";
      handle.classList.add("active");
      document.body.style.cursor = axis === "x" ? "col-resize" : "row-resize";
      document.body.style.userSelect = "none";

      function onMove(e2) {
        var current = axis === "x" ? e2.clientX : e2.clientY;
        var delta = (current - startPos) * sign;
        var newSize = Math.max(0, startSize + delta);
        if (newSize < min) {
          target.style.display = "none";
          if (axis === "x") target.style.width = "0px";
          else target.style.height = "0px";
        } else {
          target.style.display = "";
          if (axis === "x") target.style.width = Math.min(newSize, max) + "px";
          else target.style.height = Math.min(newSize, max) + "px";
        }
      }

      function onUp() {
        handle.classList.remove("active");
        target.style.transition = "";
        document.body.style.cursor = "";
        document.body.style.userSelect = "";
        document.removeEventListener("mousemove", onMove);
        document.removeEventListener("mouseup", onUp);
      }

      document.addEventListener("mousemove", onMove);
      document.addEventListener("mouseup", onUp);
    });
  }

  function insertResizeHandles() {
    var sidebar = qs(".studio-sidebar");
    var inspector = qs(".studio-inspector");
    var chatPanel = qs(".chat-panel");

    if (sidebar && sidebar.nextElementSibling) {
      var h1 = document.createElement("div");
      h1.className = "resize-handle resize-handle-h";
      sidebar.parentNode.insertBefore(h1, sidebar.nextSibling);
      dragResize(h1, sidebar, "x", 1, 60, 500);
    }

    if (inspector) {
      var h2 = document.createElement("div");
      h2.className = "resize-handle resize-handle-h";
      inspector.parentNode.insertBefore(h2, inspector);
      dragResize(h2, inspector, "x", -1, 60, 500);
    }

    if (chatPanel) {
      var hv = document.createElement("div");
      hv.className = "resize-handle resize-handle-v";
      chatPanel.parentNode.insertBefore(hv, chatPanel);
      dragResize(hv, chatPanel, "y", -1, 50, 600);
    }
  }

  insertResizeHandles();

  // --- Panel Toggle Buttons (VS Code style) ---
  // SVG icons matching the screenshot: left panel, bottom panel, right panel
  var panelIcons = {
    left: '<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><rect x="1" y="1" width="14" height="14" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/><rect x="1" y="1" width="5" height="14" rx="1" fill="currentColor" opacity="0.4"/></svg>',
    bottom: '<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><rect x="1" y="1" width="14" height="14" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/><rect x="1" y="10" width="14" height="5" rx="1" fill="currentColor" opacity="0.4"/></svg>',
    right: '<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><rect x="1" y="1" width="14" height="14" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/><rect x="10" y="1" width="5" height="14" rx="1" fill="currentColor" opacity="0.4"/></svg>'
  };

  function addPanelToggles() {
    var headerRight = qs(".header-right");
    if (!headerRight) return;

    var togglesDiv = document.createElement("div");
    togglesDiv.className = "panel-toggles";

    var panels = [
      { sel: ".studio-sidebar", icon: panelIcons.left, title: "Explorer (left panel)" },
      { sel: ".chat-panel", icon: panelIcons.bottom, title: "Chat (bottom panel)" },
      { sel: ".studio-inspector", icon: panelIcons.right, title: "Inspector (right panel)" }
    ];

    panels.forEach(function(p) {
      var btn = document.createElement("button");
      btn.className = "panel-toggle-btn active";
      btn.innerHTML = p.icon;
      btn.title = p.title;
      btn.addEventListener("click", function() {
        var el = qs(p.sel);
        if (!el) return;
        if (el.style.display === "none") {
          el.style.display = "";
          btn.classList.add("active");
        } else {
          el.style.display = "none";
          btn.classList.remove("active");
        }
      });
      togglesDiv.appendChild(btn);
    });

    var themeBtn2 = headerRight.querySelector(".theme-toggle");
    headerRight.insertBefore(togglesDiv, themeBtn2);
  }

  addPanelToggles();

  // --- Expose navigation for breadcrumb onclick ---
  window.__studio = {
    nav: function(level, module, func, block) {
      navigateTo(level, module, func, block);
    }
  };

  // --- Initial load: fetch context data and render ---
  navigateTo("context");
})();
