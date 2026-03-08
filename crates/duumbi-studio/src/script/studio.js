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

  // --- Theme toggle (only affects C4 graph area) ---
  var graphContainer = null;
  var graphTheme = "dark";
  var themeBtn = qs(".theme-toggle");
  if (themeBtn) {
    themeBtn.addEventListener("click", function() {
      graphContainer = graphContainer || qs(".graph-canvas-container");
      if (!graphContainer) return;
      if (graphTheme === "dark") {
        graphTheme = "light";
        graphContainer.classList.add("canvas-light");
        themeBtn.textContent = "\u{1F319}";
        themeBtn.title = "Dark canvas";
      } else {
        graphTheme = "dark";
        graphContainer.classList.remove("canvas-light");
        themeBtn.textContent = "\u{2600}";
        themeBtn.title = "Light canvas";
      }
    });
  }

  // --- Sidebar toggle ---
  const sidebarToggle = qs(".sidebar-toggle");
  if (sidebarToggle) {
    sidebarToggle.addEventListener("click", function() {
      const content = qs(".sidebar-content");
      if (content) {
        const hidden = content.style.display === "none";
        content.style.display = hidden ? "block" : "none";
        sidebarToggle.textContent = hidden ? "<" : ">";
      }
    });
  }

  // --- Graph rendering ---
  function renderGraph(data) {
    const svg = qs(".graph-canvas");
    if (!svg) return;

    const g = svg.querySelector("g");
    if (!g) return;

    // Clear existing content (keep defs)
    g.innerHTML = "";

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

        const rect = document.createElementNS("http://www.w3.org/2000/svg", "rect");
        rect.setAttribute("x", node.x - node.width / 2);
        rect.setAttribute("y", node.y - node.height / 2);
        rect.setAttribute("width", node.width);
        rect.setAttribute("height", node.height);
        rect.setAttribute("rx", "8");
        rect.setAttribute("ry", "8");
        rect.setAttribute("class", "node-rect");
        group.appendChild(rect);

        const label = document.createElementNS("http://www.w3.org/2000/svg", "text");
        label.setAttribute("x", node.x);
        label.setAttribute("y", node.y + 4);
        label.setAttribute("text-anchor", "middle");
        label.setAttribute("class", "node-label");
        label.textContent = node.label;
        group.appendChild(label);

        if (node.badge) {
          var badge = document.createElementNS("http://www.w3.org/2000/svg", "text");
          badge.setAttribute("x", node.x);
          badge.setAttribute("y", node.y + node.height / 2 - 6);
          badge.setAttribute("text-anchor", "middle");
          badge.setAttribute("class", "node-badge");
          badge.textContent = node.badge;
          group.appendChild(badge);
        }

        // Entry/exit markers: small colored indicator
        if (node.node_type && node.node_type.indexOf("entry") !== -1) {
          var marker = document.createElementNS("http://www.w3.org/2000/svg", "circle");
          marker.setAttribute("cx", node.x - node.width / 2 + 10);
          marker.setAttribute("cy", node.y - node.height / 2 + 10);
          marker.setAttribute("r", "5");
          marker.setAttribute("fill", "#3fb950");
          marker.setAttribute("class", "entry-marker");
          group.appendChild(marker);
          var mt = document.createElementNS("http://www.w3.org/2000/svg", "text");
          mt.setAttribute("x", node.x - node.width / 2 + 20);
          mt.setAttribute("y", node.y - node.height / 2 + 14);
          mt.setAttribute("class", "node-badge");
          mt.setAttribute("fill", "#3fb950");
          mt.setAttribute("font-size", "9");
          mt.textContent = "IN";
          group.appendChild(mt);
        }
        if (node.node_type && node.node_type.indexOf("exit") !== -1) {
          var emarker = document.createElementNS("http://www.w3.org/2000/svg", "circle");
          emarker.setAttribute("cx", node.x - node.width / 2 + 10);
          emarker.setAttribute("cy", node.y - node.height / 2 + 10);
          emarker.setAttribute("r", "5");
          emarker.setAttribute("fill", "#d29922");
          emarker.setAttribute("class", "exit-marker");
          group.appendChild(emarker);
          var et = document.createElementNS("http://www.w3.org/2000/svg", "text");
          et.setAttribute("x", node.x - node.width / 2 + 20);
          et.setAttribute("y", node.y - node.height / 2 + 14);
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

    // Update module list from context data
    if (data.modules) {
      const moduleList = qs(".module-tree");
      if (moduleList) {
        moduleList.innerHTML = "";
        data.modules.forEach(function(mod) {
          const li = document.createElement("li");
          li.className = "module-item";
          li.innerHTML = '<span class="module-icon">></span><span class="module-name">' +
            mod + '</span>';
          li.style.cursor = "pointer";
          li.addEventListener("click", function() {
            navigateTo("container", mod);
          });
          moduleList.appendChild(li);
        });
      }
    }
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

  // --- Single click on function/block also drills down (more intuitive) ---
  function onNodeClick(node) {
    // Update inspector
    updateInspector(node);

    // Also drill down on single click for navigation
    if (currentLevel === "context" && node.node_type === "module") {
      navigateTo("container", node.id);
    } else if (currentLevel === "container" && node.node_type === "function") {
      navigateTo("component", currentModule, node.id);
    } else if (currentLevel === "component" && node.node_type === "block") {
      navigateTo("code", currentModule, currentFunction, node.id);
    }
  }

  // --- Navigation ---
  function navigateTo(level, module, func, block) {
    currentLevel = level;
    currentModule = module || null;
    currentFunction = func || null;
    currentBlock = block || null;

    // Update C4 tabs
    qsa(".c4-tab").forEach(function(tab) {
      tab.classList.toggle("active", tab.textContent.toLowerCase() === level);
    });

    // Update breadcrumb
    updateBreadcrumb();

    // Reset zoom/pan on navigation
    svgScale = 1;
    svgPanX = 0;
    svgPanY = 0;
    updateGrid();

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
        if (data.error) {
          console.error("API error:", data.error);
          return;
        }
        renderGraph(data);
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
  var GRID_BASE = 24;

  // Snap value to nearest grid point
  function snapToGrid(val) {
    return Math.round(val / GRID_BASE) * GRID_BASE;
  }

  // Update CSS dot grid to match zoom + pan
  function updateGrid() {
    var container = qs(".graph-canvas-container");
    if (!container) return;
    var gSize = GRID_BASE * svgScale;
    container.style.backgroundSize = gSize + "px " + gSize + "px";
    container.style.backgroundPosition = svgPanX + "px " + svgPanY + "px";
  }

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

  if (svgCanvas) {
    svgCanvas.addEventListener("wheel", function(e) {
      e.preventDefault();
      var factor = 1 + Math.min(Math.abs(e.deltaY), 50) * 0.002;
      var delta = e.deltaY > 0 ? 1 / factor : factor;
      var newScale = svgScale * delta;
      newScale = Math.max(0.2, Math.min(4, newScale));

      // Zoom toward mouse position
      var rect = svgCanvas.getBoundingClientRect();
      var mx = e.clientX - rect.left;
      var my = e.clientY - rect.top;
      svgPanX = mx - (mx - svgPanX) * (newScale / svgScale);
      svgPanY = my - (my - svgPanY) * (newScale / svgScale);
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
      svgPanX += (e.clientX - svgLastX);
      svgPanY += (e.clientY - svgLastY);
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

  // Compute connection point on the border of a node rect toward a target point.
  function borderPoint(nodePos, tx, ty) {
    var cx = nodePos.cx, cy = nodePos.cy;
    var hw = nodePos.w / 2, hh = nodePos.h / 2;
    var dx = tx - cx, dy = ty - cy;
    if (dx === 0 && dy === 0) return { x: cx, y: cy + hh };
    // Find intersection with rect border
    var absDx = Math.abs(dx), absDy = Math.abs(dy);
    var scale;
    if (absDx * hh > absDy * hw) {
      scale = hw / absDx;
    } else {
      scale = hh / absDy;
    }
    return { x: cx + dx * scale, y: cy + dy * scale };
  }

  // Recompute all edge paths and connection dots from current node positions.
  function updateEdges() {
    var g = svgCanvas ? svgCanvas.querySelector("g") : null;
    if (!g) return;

    // Remove old connection dots
    g.querySelectorAll(".conn-dot").forEach(function(d) { d.remove(); });

    g.querySelectorAll(".edge-path").forEach(function(pathEl) {
      var srcId = pathEl.dataset.edgeSrc;
      var tgtId = pathEl.dataset.edgeTgt;
      var src = nodePositions[srcId];
      var tgt = nodePositions[tgtId];
      if (!src || !tgt) return;

      // Connection points on borders
      var sp = borderPoint(src, tgt.cx, tgt.cy);
      var tp = borderPoint(tgt, src.cx, src.cy);

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

      // Connection dots
      var dot1 = document.createElementNS("http://www.w3.org/2000/svg", "circle");
      dot1.setAttribute("cx", sp.x);
      dot1.setAttribute("cy", sp.y);
      dot1.setAttribute("r", "3");
      dot1.setAttribute("class", "conn-dot");
      g.appendChild(dot1);

      var dot2 = document.createElementNS("http://www.w3.org/2000/svg", "circle");
      dot2.setAttribute("cx", tp.x);
      dot2.setAttribute("cy", tp.y);
      dot2.setAttribute("r", "3");
      dot2.setAttribute("class", "conn-dot");
      g.appendChild(dot2);
    });
  }

  // --- Draggable nodes (using group transform) ---
  var dragNode = null;
  var dragStartX = 0, dragStartY = 0;
  var dragOffsetX = 0, dragOffsetY = 0;

  function enableNodeDrag() {
    if (!svgCanvas) return;

    svgCanvas.addEventListener("mousedown", function(e) {
      var nodeGroup = e.target.closest(".graph-node");
      if (!nodeGroup) return;
      e.stopPropagation();
      dragNode = nodeGroup;

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

        // Remove group transform and update child attributes directly
        dragNode.removeAttribute("transform");

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

        // Update stored position
        var nodeId = dragNode.dataset.nodeId;
        nodePositions[nodeId] = { cx: newCx, cy: newCy, w: w, h: h };
      }

      updateEdges();
      dragNode.style.opacity = "";
      dragNode = null;
    });
  }

  enableNodeDrag();

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
        currentLayout = l.id;
        qsa(".layout-btn").forEach(function(b) { b.classList.remove("active"); });
        btn.classList.add("active");
        // Re-fetch with layout param
        reloadCurrentView();
      });
      toolbar.appendChild(btn);
    });

    container.appendChild(toolbar);
  }

  function reloadCurrentView() {
    navigateTo(currentLevel, currentModule, currentFunction, currentBlock);
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
