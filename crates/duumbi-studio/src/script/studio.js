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

  // Code view state
  var codeViewActive = false;
  var lastGraphData = null;

  // Filter state
  var activeFilters = {};
  var filterPopupVisible = false;
  var TYPE_COLORS = {
    person: "#58a6ff", system: "#388bfd", external: "#8b949e",
    container: "#a371f7", component: "#3fb950", boundary: "#30363d",
    module: "#388bfd", "function": "#a371f7", block: "#3fb950",
    "component-dead": "#6e7681", "component-sub": "#d2a8ff"
  };

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

  // --- Render a single graph node into the SVG group ---
  function renderSingleNode(g, node) {
    const group = document.createElementNS("http://www.w3.org/2000/svg", "g");
    group.setAttribute("class", "graph-node node-" + (node.node_type || "default"));
    group.style.cursor = "pointer";
    group.dataset.nodeId = node.id;
    group.dataset.nodeType = node.node_type;
    if (node.node_type && node.node_type.indexOf("entry") !== -1) {
      group.dataset.entry = "true";
    }
    if (node.node_type && node.node_type.indexOf("exit") !== -1) {
      group.dataset.exit = "true";
    }

    var nx = snapToGrid(node.x);
    var ny = snapToGrid(node.y);

    const rect = document.createElementNS("http://www.w3.org/2000/svg", "rect");
    rect.setAttribute("x", nx - node.width / 2);
    rect.setAttribute("y", ny - node.height / 2);
    rect.setAttribute("width", node.width);
    rect.setAttribute("height", node.height);
    var rx = "8";
    switch (node.node_type) {
      case "person": rx = String(node.width / 2); break;
      case "system": case "container": case "component": rx = "12"; break;
      case "boundary": rx = "16"; break;
      case "external": rx = "2"; break;
      case "component-dead": case "component-sub": rx = "8"; break;
      case "block": rx = "4"; break;
      case "Const": case "ConstF64": case "ConstBool": rx = String(node.width / 2); break;
    }
    rect.setAttribute("rx", rx);
    rect.setAttribute("ry", rx);
    rect.setAttribute("class", "node-rect");
    group.appendChild(rect);

    const label = document.createElementNS("http://www.w3.org/2000/svg", "text");
    label.setAttribute("x", nx);
    label.setAttribute("y", node.badge ? ny - 4 : ny + 4);
    label.setAttribute("text-anchor", "middle");
    label.setAttribute("class", "node-label");
    label.textContent = node.label;
    group.appendChild(label);

    if (node.badge) {
      var badge = document.createElementNS("http://www.w3.org/2000/svg", "text");
      badge.setAttribute("x", nx);
      badge.setAttribute("y", ny + 14);
      badge.setAttribute("text-anchor", "middle");
      badge.setAttribute("class", "node-badge");
      badge.textContent = node.badge;
      group.appendChild(badge);
    }

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

    group.addEventListener("click", function() { onNodeClick(node); });
    group.addEventListener("dblclick", function() { onNodeDblClick(node); });

    g.appendChild(group);
  }

  // --- Graph rendering ---
  function renderGraph(data) {
    lastGraphData = data;
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
          text.setAttribute("y", edge.label_y);
          text.setAttribute("text-anchor", "middle");
          text.setAttribute("dominant-baseline", "central");
          text.setAttribute("class", "edge-label");
          text.textContent = edge.label;
          g.appendChild(text);
        }
      });
    }

    // Separate boundary nodes from regular nodes
    var boundaryNodes = [];
    var regularNodes = [];
    if (data.nodes) {
      data.nodes.forEach(function(node) {
        if (node.node_type === "boundary") {
          boundaryNodes.push(node);
        } else {
          regularNodes.push(node);
        }
      });
    }

    // Render regular nodes first
    regularNodes.forEach(function(node) {
      renderSingleNode(g, node);
    });

    // Render boundary nodes AFTER regular nodes, sized to enclose their children
    boundaryNodes.forEach(function(bNode) {
      // Find child nodes: containers that are "inside" this boundary
      var childIds = [];
      regularNodes.forEach(function(n) {
        if (n.node_type === "container") childIds.push(n.id);
      });

      // Compute bounding box of children
      var pad = 40;
      var minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
      childIds.forEach(function(cid) {
        var el = qs('[data-node-id="' + cid + '"]');
        if (!el) return;
        var r = el.querySelector(".node-rect");
        if (!r) return;
        var rx = parseFloat(r.getAttribute("x"));
        var ry = parseFloat(r.getAttribute("y"));
        var rw = parseFloat(r.getAttribute("width"));
        var rh = parseFloat(r.getAttribute("height"));
        if (rx < minX) minX = rx;
        if (ry < minY) minY = ry;
        if (rx + rw > maxX) maxX = rx + rw;
        if (ry + rh > maxY) maxY = ry + rh;
      });

      if (minX === Infinity) {
        // No children found — render as normal node
        renderSingleNode(g, bNode);
        return;
      }

      // Compute boundary rect around children (extra 28px top for title)
      var bx = minX - pad;
      var by = minY - pad - 28;
      var bw = maxX - minX + 2 * pad;
      var bh = maxY - minY + 2 * pad + 28;

      var group = document.createElementNS("http://www.w3.org/2000/svg", "g");
      group.setAttribute("class", "graph-node node-boundary");
      group.style.cursor = "pointer";
      group.dataset.nodeId = bNode.id;
      group.dataset.nodeType = "boundary";
      group.dataset.children = childIds.join(",");

      var rect = document.createElementNS("http://www.w3.org/2000/svg", "rect");
      rect.setAttribute("x", bx);
      rect.setAttribute("y", by);
      rect.setAttribute("width", bw);
      rect.setAttribute("height", bh);
      rect.setAttribute("rx", "16");
      rect.setAttribute("ry", "16");
      rect.setAttribute("class", "node-rect");
      group.appendChild(rect);

      // Label at top-left of boundary
      var lbl = document.createElementNS("http://www.w3.org/2000/svg", "text");
      lbl.setAttribute("x", bx + 16);
      lbl.setAttribute("y", by + 20);
      lbl.setAttribute("text-anchor", "start");
      lbl.setAttribute("class", "node-label boundary-label");
      lbl.textContent = bNode.label;
      group.appendChild(lbl);

      group.addEventListener("click", function() { onNodeClick(bNode); });
      group.addEventListener("dblclick", function() { onNodeDblClick(bNode); });

      // Insert boundary BEFORE the child nodes so it renders behind them
      var firstChild = qs('[data-node-id="' + childIds[0] + '"]');
      if (firstChild) {
        g.insertBefore(group, firstChild);
      } else {
        g.appendChild(group);
      }

      // Store boundary position
      nodePositions[bNode.id] = {
        cx: bx + bw / 2, cy: by + bh / 2, w: bw, h: bh
      };
    });

    // Push external nodes away from boundary rects to ensure spacing
    boundaryNodes.forEach(function(bNode) {
      var bEl = qs('[data-node-id="' + bNode.id + '"]');
      if (!bEl) return;
      var bRect = bEl.querySelector(".node-rect");
      if (!bRect) return;
      var bbx = parseFloat(bRect.getAttribute("x"));
      var bby = parseFloat(bRect.getAttribute("y"));
      var bbw = parseFloat(bRect.getAttribute("width"));
      var bbh = parseFloat(bRect.getAttribute("height"));
      var MARGIN = 30;

      regularNodes.forEach(function(n) {
        if (n.node_type === "container") return;
        var nEl = qs('[data-node-id="' + n.id + '"]');
        if (!nEl) return;
        var nRect = nEl.querySelector(".node-rect");
        if (!nRect) return;
        var nx = parseFloat(nRect.getAttribute("x"));
        var ny = parseFloat(nRect.getAttribute("y"));
        var nw = parseFloat(nRect.getAttribute("width"));
        var nh = parseFloat(nRect.getAttribute("height"));

        // Check horizontal overlap
        var hOverlap = nx < bbx + bbw && nx + nw > bbx;
        if (!hOverlap) return;

        // Node is above boundary — push up if too close
        if (ny + nh > bby - MARGIN && ny + nh <= bby + bbh / 2) {
          var shiftY = (bby - MARGIN) - (ny + nh);
          nRect.setAttribute("y", ny + shiftY);
          // Move labels too
          var labels = nEl.querySelectorAll("text");
          labels.forEach(function(lbl) {
            var ly = parseFloat(lbl.getAttribute("y"));
            lbl.setAttribute("y", ly + shiftY);
          });
        }
        // Node is below boundary — push down if too close
        if (ny < bby + bbh + MARGIN && ny >= bby + bbh / 2) {
          var newY = bby + bbh + MARGIN;
          var shiftY2 = newY - ny;
          nRect.setAttribute("y", ny + shiftY2);
          var labels2 = nEl.querySelectorAll("text");
          labels2.forEach(function(lbl2) {
            var ly2 = parseFloat(lbl2.getAttribute("y"));
            lbl2.setAttribute("y", ly2 + shiftY2);
          });
        }
      });
    });

    // Store positions and compute proper edge routing with connection dots
    storeNodePositions();
    updateEdges();

    // Cache C4 hierarchy data for sidebar tree
    if (data.modules) {
      sidebarModules = data.modules;
    }
    if (currentLevel === "context" && data.nodes) {
      // Cache context-level nodes (person, system, external) per module
      sidebarModules.forEach(function(mod) {
        sidebarContextNodes[mod] = data.nodes
          .filter(function(n) { return n.node_type !== "boundary" && n.node_type !== "system"; })
          .map(function(n) { return { id: n.id, label: n.label, type: n.node_type }; });
      });
    }
    if (currentLevel === "container" && data.nodes) {
      // Cache containers (skip boundary nodes)
      sidebarContainers[currentModule] = data.nodes
        .filter(function(n) { return n.node_type !== "boundary"; })
        .map(function(n) { return { id: n.id, label: n.label, type: n.node_type }; });
    }
    if (currentLevel === "component" && data.nodes) {
      // Cache components (active/dead functions, skip boundaries)
      sidebarComponents[currentModule] = data.nodes
        .filter(function(n) { return n.node_type !== "boundary"; })
        .map(function(n) { return { id: n.id, label: n.label, type: n.node_type }; });
    }
    if (currentLevel === "code" && data.nodes) {
      // Cache code-level ops for potential future expansion
      var codeKey = currentModule + "/" + currentFunction + "/" + currentBlock;
      sidebarCodeOps[codeKey] = data.nodes
        .map(function(n) { return { id: n.id, label: n.label }; });
    }

    updateSidebarTree();
    applyFilters();
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
    // C4 Context → Container (click on software system)
    if (currentLevel === "context" && (node.node_type === "system" || node.node_type === "module")) {
      navigateTo("container", "app/main");
    // C4 Container → Component (only the native binary drills down)
    } else if (currentLevel === "container" && node.id === "container:binary") {
      navigateTo("component", currentModule || "app/main", "main");
    } else if (currentLevel === "container" && node.node_type === "function") {
      navigateTo("component", currentModule || "app/main", node.id);
    // C4 Component → Code (click on active component/function)
    } else if (currentLevel === "component" && (node.node_type === "component" || node.node_type === "block")) {
      // Extract function name from id: "component:main" → "main"
      var fnName = node.id.replace(/^component:/, "");
      navigateTo("code", currentModule || "app/main", fnName, "entry");
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
      var isBnd = group.dataset.nodeType === "boundary";
      group.querySelectorAll("text").forEach(function(t) {
        if (t.classList.contains("node-label")) {
          if (isBnd) {
            // Boundary label stays at top-left
            t.setAttribute("x", cx - w / 2 + 16);
            t.setAttribute("y", cy - h / 2 + 20);
          } else {
            t.setAttribute("x", cx);
            t.setAttribute("y", cy - 4);
          }
        } else if (t.classList.contains("node-badge") && !t.hasAttribute("font-size")) {
          t.setAttribute("x", cx); t.setAttribute("y", cy + 14);
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
  var sidebarModules = [];       // cached module names from context level
  var sidebarContextNodes = {};  // moduleId → [{id, label, type}] from context level (person, external, etc.)
  var sidebarContainers = {};    // moduleId → [{id, label, type}] from container level
  var sidebarComponents = {};    // moduleId → [{id, label, type}] from component level
  var sidebarCodeOps = {};       // "mod/fn/block" → [{id, label}] from code level

  // C4 icon+color map for sidebar items
  function c4Icon(type) {
    switch (type) {
      case "person":          return { icon: "\uD83D\uDC64", color: "#5b9bd5" }; // 👤 blue
      case "system":          return { icon: "\u2B22",   color: "#5b9bd5" }; // ⬢ blue
      case "external":        return { icon: "\u2B21",   color: "#999" };    // ⬡ grey
      case "container:binary":return { icon: "\u25A0",   color: "#2ecc71" }; // ■ green
      case "container":       return { icon: "\u25A1",   color: "#7f8c8d" }; // □ grey
      case "component":       return { icon: "\u25C6",   color: "#3498db" }; // ◆ blue
      case "component:dead":  return { icon: "\u25C7",   color: "#666" };    // ◇ dim
      default:                return { icon: "\u25B8",   color: "#888" };    // ▸ fallback
    }
  }

  var chevronRight = '<svg viewBox="0 0 16 16"><polyline points="6 4 10 8 6 12"/></svg>';
  var chevronDown  = '<svg viewBox="0 0 16 16"><polyline points="4 6 8 10 12 6"/></svg>';

  function sidebarItem(label, type, depth, isActive, onClick, expandable) {
    var li = document.createElement("li");
    var depthClass = depth === 1 ? " tree-child" : depth === 2 ? " tree-child tree-child-2" : depth === 3 ? " tree-child tree-child-2 tree-child-3" : "";
    li.className = "module-item" + depthClass + (isActive ? " tree-active" : "");
    var ic = c4Icon(type);
    var arrowHtml = expandable ? '<span class="tree-arrow">' + (isActive ? chevronDown : chevronRight) + '</span>' : "";
    li.innerHTML = arrowHtml +
      '<span class="tree-icon" style="color:' + ic.color + '">' + ic.icon + '</span>' +
      '<span class="module-name">' + label + '</span>';
    if (onClick) {
      li.style.cursor = "pointer";
      li.addEventListener("click", onClick);
    } else {
      li.style.opacity = "0.5";
    }
    return li;
  }

  function updateSidebarTree() {
    var tree = qs(".module-tree");
    if (!tree) return;
    tree.innerHTML = "";

    // Level 0: Modules (always visible)
    sidebarModules.forEach(function(mod) {
      var isActiveModule = (currentModule === mod);
      var isExpanded = isActiveModule && currentLevel !== "context";

      var li = document.createElement("li");
      li.className = "module-item" + (isActiveModule ? " tree-active" : "");
      var arrow = isExpanded ? chevronDown : chevronRight;
      li.innerHTML = '<span class="tree-arrow">' + arrow + '</span>' +
        '<span class="tree-icon" style="color:#5b9bd5">\u2B22</span>' +
        '<span class="module-name">' + mod + '</span>';
      li.style.cursor = "pointer";
      li.addEventListener("click", function() {
        // Toggle: if already expanded at container level, collapse back to context
        if (isExpanded && currentLevel === "container") {
          navigateTo("context", mod);
        } else {
          navigateTo("container", mod);
        }
      });
      tree.appendChild(li);

      if (!isExpanded) return;

      // Level 1: Containers + context nodes (visible when drilled into a module)
      var containers = sidebarContainers[mod] || [];
      var contextNodes = sidebarContextNodes[mod] || [];

      // Show only drillable containers (skip non-clickable items)
      containers.forEach(function(ct) {
        var isDrillable = (ct.id === "container:binary");
        if (!isDrillable) return; // hide non-clickable containers

        var isActiveCt = (currentLevel === "component" || currentLevel === "code");

        tree.appendChild(sidebarItem(ct.label, ct.type, 1, isActiveCt, function() {
          // Toggle: if already expanded at component level, collapse back to container
          if (isActiveCt && currentLevel === "component") {
            navigateTo("container", mod);
          } else {
            navigateTo("component", mod, "main");
          }
        }, true)); // expandable

        // Level 2: Only show main() — the entry point that has a Code view
        if (isActiveCt && sidebarComponents[mod]) {
          sidebarComponents[mod]
            .filter(function(comp) { return comp.id === "component:main"; })
            .forEach(function(comp) {
              var fnName = comp.id.replace(/^component:/, "");
              var isActiveComp = (currentLevel === "code" && currentFunction === fnName);

              tree.appendChild(sidebarItem(comp.label, "component", 2, isActiveComp, function() {
                // Toggle: if already at code level for this function, collapse back to component
                if (isActiveComp) {
                  navigateTo("component", mod, "main");
                } else {
                  navigateTo("code", mod, fnName, "entry");
                }
              }, false));
            });
        }
      });
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

    // Add code/graph toggle at code level
    if (currentLevel === "code" && currentBlock) {
      var toggleIcon = codeViewActive
        ? '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="8" cy="8" r="3"/><line x1="3" y1="3" x2="5.5" y2="5.5"/><line x1="13" y1="3" x2="10.5" y2="5.5"/><line x1="3" y1="13" x2="5.5" y2="10.5"/><line x1="13" y1="13" x2="10.5" y2="10.5"/></svg>'
        : '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"><polyline points="5,3 1,8 5,13"/><polyline points="11,3 15,8 11,13"/><line x1="10" y1="2" x2="6" y2="14"/></svg>';
      var toggleTitle = codeViewActive ? "Switch to graph view" : "Switch to code view";
      html += '<button class="breadcrumb-view-toggle" onclick="window.__studio.toggleCode()" title="' + toggleTitle + '">' + toggleIcon + '</button>';
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
        labelEl.setAttribute("y", midY);
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

      // If this is a boundary node, also move its child nodes
      if (dragNode.dataset.children) {
        var childIds = dragNode.dataset.children.split(",");
        childIds.forEach(function(cid) {
          var childEl = qs('[data-node-id="' + cid + '"]');
          if (childEl && childEl !== dragNode) {
            childEl.setAttribute("transform", "translate(" + dragOffsetX + "," + dragOffsetY + ")");
            // Update child tracked position
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
        var isBoundary = dragNode.dataset.nodeType === "boundary";
        texts.forEach(function(t) {
          if (t.classList.contains("node-label")) {
            if (isBoundary) {
              // Boundary label at top-left
              t.setAttribute("x", newCx - w / 2 + 16);
              t.setAttribute("y", newCy - h / 2 + 20);
            } else {
              t.setAttribute("x", newCx);
              t.setAttribute("y", newCy - 4);
            }
          } else if (t.classList.contains("node-badge") && !t.hasAttribute("font-size")) {
            t.setAttribute("x", newCx);
            t.setAttribute("y", newCy + 14);
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

        // If boundary, also bake transforms into child nodes
        if (dragNode.dataset.children) {
          var childIds = dragNode.dataset.children.split(",");
          childIds.forEach(function(cid) {
            var childEl = qs('[data-node-id="' + cid + '"]');
            if (!childEl || childEl === dragNode) return;
            var cr = childEl.querySelector(".node-rect");
            if (!cr) return;
            var cw = parseFloat(cr.getAttribute("width"));
            var ch = parseFloat(cr.getAttribute("height"));
            var cox = parseFloat(cr.getAttribute("x"));
            var coy = parseFloat(cr.getAttribute("y"));
            var cnx = snapToGrid(cox + cw / 2 + dragOffsetX);
            var cny = snapToGrid(coy + ch / 2 + dragOffsetY);

            childEl.querySelectorAll("*").forEach(function(el) { el.style.transition = "none"; });
            cr.setAttribute("x", cnx - cw / 2);
            cr.setAttribute("y", cny - ch / 2);
            var cTexts = childEl.querySelectorAll("text");
            var childHasBadge = Array.prototype.some.call(cTexts, function(t) {
              return t.classList.contains("node-badge");
            });
            cTexts.forEach(function(t) {
              if (t.classList.contains("node-label")) {
                t.setAttribute("x", cnx);
                // Recompute label Y from the snapped center (matching renderSingleNode rules)
                t.setAttribute("y", childHasBadge ? cny - 4 : cny + 4);
              } else if (t.classList.contains("node-badge") && !t.hasAttribute("font-size")) {
                t.setAttribute("x", cnx);
                t.setAttribute("y", cny + 14);
              }
            });
            childEl.removeAttribute("transform");
            nodePositions[cid] = { cx: cnx, cy: cny, w: cw, h: ch };
            requestAnimationFrame(function() {
              childEl.querySelectorAll("*").forEach(function(el) { el.style.transition = ""; });
            });
          });
        }
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

  // --- Filter Functions ---
  function toggleFilterPopup() {
    var existing = qs(".filter-popup");
    if (existing) {
      existing.remove();
      filterPopupVisible = false;
      return;
    }
    filterPopupVisible = true;

    // Collect unique node types from current DOM
    var types = {};
    qsa(".graph-node").forEach(function(el) {
      var t = el.dataset.nodeType;
      if (t) types[t] = true;
    });

    var container = qs(".graph-canvas-container");
    if (!container) return;

    var popup = document.createElement("div");
    popup.className = "filter-popup";

    var title = document.createElement("div");
    title.className = "filter-popup-title";
    title.textContent = "Filter by type";
    popup.appendChild(title);

    Object.keys(types).sort().forEach(function(t) {
      var row = document.createElement("label");
      row.className = "filter-row";

      var cb = document.createElement("input");
      cb.type = "checkbox";
      cb.checked = !activeFilters[t];
      cb.addEventListener("change", function() {
        if (cb.checked) {
          delete activeFilters[t];
        } else {
          activeFilters[t] = true;
        }
        applyFilters();
      });

      var dot = document.createElement("span");
      dot.className = "filter-dot";
      dot.style.background = TYPE_COLORS[t] || "#8b949e";

      var lbl = document.createElement("span");
      lbl.className = "filter-type-label";
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
      // Remove all filter classes
      qsa(".node-filtered").forEach(function(el) { el.classList.remove("node-filtered"); });
      qsa(".edge-filtered").forEach(function(el) { el.classList.remove("edge-filtered"); });
      return;
    }

    // Filter nodes
    qsa(".graph-node").forEach(function(el) {
      var t = el.dataset.nodeType;
      if (t && activeFilters[t]) {
        el.classList.add("node-filtered");
      } else {
        el.classList.remove("node-filtered");
      }
    });

    // Filter edges: if source OR target is filtered, dim the edge
    var filteredIds = {};
    qsa(".graph-node.node-filtered").forEach(function(el) {
      filteredIds[el.dataset.nodeId] = true;
    });
    // Edges are rendered as standalone .edge-path / .edge-label elements
    // (no .graph-edge wrapper), so target them directly.
    qsa(".edge-path").forEach(function(edgeEl) {
      var src = edgeEl.dataset.edgeSrc;
      var tgt = edgeEl.dataset.edgeTgt;
      if (!src || !tgt) return;
      if (filteredIds[src] || filteredIds[tgt]) {
        edgeEl.classList.add("edge-filtered");
      } else {
        edgeEl.classList.remove("edge-filtered");
      }
    });
    qsa(".edge-label").forEach(function(labelEl) {
      var src = labelEl.dataset.edgeSrc;
      var tgt = labelEl.dataset.edgeTgt;
      if (!src || !tgt) return;
      if (filteredIds[src] || filteredIds[tgt]) {
        labelEl.classList.add("edge-filtered");
      } else {
        labelEl.classList.remove("edge-filtered");
      }
    });
  }

  // Close filter popup on document click
  document.addEventListener("click", function(e) {
    if (!filterPopupVisible) return;
    var popup = qs(".filter-popup");
    var btn = qs(".filter-toggle-btn");
    if (popup && !popup.contains(e.target) && btn && !btn.contains(e.target)) {
      popup.remove();
      filterPopupVisible = false;
    }
  });

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
    var sepFilter = document.createElement("div");
    sepFilter.className = "layout-toolbar-sep";
    toolbar.appendChild(sepFilter);

    // Filter button
    var filterBtn = document.createElement("button");
    filterBtn.className = "layout-btn filter-toggle-btn";
    filterBtn.title = "Filter by type";
    filterBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"/></svg>';
    filterBtn.addEventListener("click", function(e) {
      e.stopPropagation();
      toggleFilterPopup();
    });
    toolbar.appendChild(filterBtn);

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

  // --- Extract block ops from parsed JSON-LD for code view filtering ---
  function extractBlockOps(parsed, functionName, blockLabel) {
    var functions = parsed["duumbi:functions"];
    if (!Array.isArray(functions)) return parsed;
    for (var i = 0; i < functions.length; i++) {
      var fn = functions[i];
      if (fn["duumbi:name"] !== functionName) continue;
      var blocks = fn["duumbi:blocks"];
      if (!Array.isArray(blocks)) return parsed;
      for (var j = 0; j < blocks.length; j++) {
        var block = blocks[j];
        if (block["duumbi:label"] === blockLabel) {
          return block;
        }
      }
    }
    return parsed;
  }

  // --- Code View Toggle ---
  function toggleCodeView() {
    if (codeViewActive) {
      // Switch back to graph view
      codeViewActive = false;
      var container = qs(".graph-canvas-container");
      // Remove code view, restore SVG canvas
      var codeView = qs(".code-view");
      if (codeView) codeView.remove();
      var svg = qs(".graph-canvas");
      if (svg) svg.style.display = "";
      // Show C4 tabs and layout toolbar
      var tabs = qs(".c4-tabs");
      if (tabs) tabs.style.display = "";
      var toolbar = qs(".layout-toolbar");
      if (toolbar) toolbar.style.display = "";
      // Re-render graph
      if (lastGraphData) renderGraph(lastGraphData);
      updateBreadcrumb();
    } else {
      // Switch to code view
      codeViewActive = true;
      updateBreadcrumb();
      var module = currentModule || "app/main";
      fetch("/api/source?module=" + encodeURIComponent(module))
        .then(function(r) { return r.json(); })
        .then(function(data) {
          if (data.error) { console.error("Source API error:", data.error); return; }
          var displaySource = data.source;
          if (currentLevel === "code" && currentFunction && currentBlock) {
            try {
              var parsed = JSON.parse(data.source);
              var blockObj = extractBlockOps(parsed, currentFunction, currentBlock);
              displaySource = JSON.stringify(blockObj, null, 2);
            } catch(e) {
              // fallback: show full source
            }
          }
          renderCodeView(displaySource);
        })
        .catch(function(e) { console.error("Source fetch error:", e); });
    }
  }

  function escapeHtml(str) {
    return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
  }

  function renderCodeView(source) {
    // Hide C4 tabs and layout toolbar
    var tabs = qs(".c4-tabs");
    if (tabs) tabs.style.display = "none";
    var toolbar = qs(".layout-toolbar");
    if (toolbar) toolbar.style.display = "none";

    // Hide SVG canvas
    var svg = qs(".graph-canvas");
    if (svg) svg.style.display = "none";

    // Remove existing code view
    var existing = qs(".code-view");
    if (existing) existing.remove();

    // Pretty-print JSON
    var pretty;
    try {
      pretty = JSON.stringify(JSON.parse(source), null, 2);
    } catch(e) {
      pretty = source;
    }

    var lines = pretty.split("\n");
    var html = '';

    // Build foldable line map: for each line with { or [, find its closing pair
    var foldTargets = {}; // lineIndex -> closingLineIndex
    for (var i = 0; i < lines.length; i++) {
      var trimmed = lines[i].trimEnd();
      var lastChar = trimmed[trimmed.length - 1];
      // Check for opening bracket (possibly after a key)
      if (lastChar === '{' || lastChar === '[') {
        var openChar = lastChar;
        var closeChar = openChar === '{' ? '}' : ']';
        var depth = 1;
        for (var j = i + 1; j < lines.length; j++) {
          var lt = lines[j].trimStart();
          for (var c = 0; c < lt.length; c++) {
            if (lt[c] === openChar) depth++;
            else if (lt[c] === closeChar) {
              depth--;
              if (depth === 0) {
                if (j > i + 1) foldTargets[i] = j;
                break;
              }
            }
          }
          if (depth === 0) break;
        }
      }
    }

    for (var i = 0; i < lines.length; i++) {
      var line = lines[i];
      var lineNum = i + 1;
      var hasFold = foldTargets.hasOwnProperty(i);
      var foldAttr = hasFold
        ? ' data-fold-start="' + i + '" data-fold-end="' + foldTargets[i] + '"'
        : '';
      var foldMarker = hasFold
        ? '<span class="code-fold-marker" data-fold="' + i + '">&#9660;</span>'
        : '<span class="code-fold-marker-spacer"></span>';

      // Syntax highlighting
      var highlighted = highlightJson(escapeHtml(line));

      html += '<div class="code-line" data-line="' + i + '"' + foldAttr + '>'
        + '<span class="line-number">' + lineNum + '</span>'
        + foldMarker
        + '<span class="line-content">' + highlighted + '</span>'
        + '</div>';
    }

    var container = qs(".graph-canvas-container");
    var codeDiv = document.createElement("div");
    codeDiv.className = "code-view";
    codeDiv.innerHTML = html;
    container.appendChild(codeDiv);

    // Fold/unfold click handlers
    codeDiv.addEventListener("click", function(e) {
      var marker = e.target.closest(".code-fold-marker");
      if (!marker) return;
      var foldIdx = parseInt(marker.dataset.fold);
      var startLine = foldIdx;
      var endLine = foldTargets[foldIdx];
      if (endLine === undefined) return;

      var isCollapsed = marker.classList.contains("collapsed");
      for (var k = startLine + 1; k < endLine; k++) {
        var lineEl = codeDiv.querySelector('.code-line[data-line="' + k + '"]');
        if (lineEl) lineEl.style.display = isCollapsed ? "" : "none";
      }
      marker.classList.toggle("collapsed");
      marker.innerHTML = isCollapsed ? "&#9660;" : "&#9654;";
    });
  }

  function highlightJson(escaped) {
    // Keywords (before string matching, only standalone values after colon, not inside words)
    escaped = escaped.replace(/:\s*(null|true|false)(?!\w)/g, ': <span class="code-keyword">$1</span>');
    // Numbers (standalone values after colon)
    escaped = escaped.replace(/:\s*(-?\d+\.?\d*)(\s*[,\r\n]?)/g, ': <span class="code-number">$1</span>$2');
    // Keys: "something": — entire quoted string before colon
    escaped = escaped.replace(/(&quot;)((?:@[\w:]+|[\w:@\-./]+))(&quot;)\s*:/g,
      '<span class="code-key">$1$2$3</span>:');
    // String values (after colon): "..."
    escaped = escaped.replace(/:\s*(&quot;)(.*?)(&quot;)/g,
      ': <span class="code-string">$1$2$3</span>');
    // Remaining quoted strings (in arrays, not already wrapped in a span)
    escaped = escaped.replace(/(&quot;)((?!<\/span>).*?)(&quot;)(?![^<]*<\/span>)/g,
      '<span class="code-string">$1$2$3</span>');
    return escaped;
  }

  // Reset code view when navigating away from code level
  var _origNavigateTo = navigateTo;
  navigateTo = function(level, module, func, block, restoreLayout) {
    if (codeViewActive && level !== "code") {
      codeViewActive = false;
      var codeView = qs(".code-view");
      if (codeView) codeView.remove();
      var svg = qs(".graph-canvas");
      if (svg) svg.style.display = "";
      var tabs = qs(".c4-tabs");
      if (tabs) tabs.style.display = "";
      var toolbar = qs(".layout-toolbar");
      if (toolbar) toolbar.style.display = "";
    }
    _origNavigateTo(level, module, func, block, restoreLayout);
  };

  // --- Expose navigation for breadcrumb onclick ---
  window.__studio = {
    nav: function(level, module, func, block) {
      navigateTo(level, module, func, block);
    },
    toggleCode: function() {
      toggleCodeView();
    }
  };

  // --- Initial load: fetch context data and render ---
  navigateTo("context");
})();
