// duumbi Graph Visualizer — Cytoscape.js frontend
(function () {
  "use strict";

  var cy = null;
  var ws = null;
  var reconnectTimer = null;

  // --- Cytoscape style ---
  var cytoscapeStyle = [
    // Function compound node
    {
      selector: 'node[nodeType="function"]',
      style: {
        "shape": "round-rectangle",
        "background-color": "#0d1117",
        "border-color": "#58a6ff",
        "border-width": 2,
        "label": "data(label)",
        "font-size": "12px",
        "color": "#58a6ff",
        "text-valign": "top",
        "text-halign": "center",
        "text-margin-y": -6,
        "padding": "16px",
        "compound-sizing-wrt-labels": "include"
      }
    },
    // Block compound node
    {
      selector: 'node[nodeType="block"]',
      style: {
        "shape": "round-rectangle",
        "background-color": "#161b22",
        "border-color": "#30363d",
        "border-width": 1,
        "border-style": "dashed",
        "label": "data(label)",
        "font-size": "10px",
        "color": "#8b949e",
        "text-valign": "top",
        "text-halign": "center",
        "text-margin-y": -4,
        "padding": "12px"
      }
    },
    // Op nodes — base style
    {
      selector: 'node[nodeType="op"]',
      style: {
        "shape": "round-rectangle",
        "width": "label",
        "height": "30px",
        "padding": "8px",
        "label": "data(label)",
        "font-size": "11px",
        "color": "#c9d1d9",
        "text-valign": "center",
        "text-halign": "center",
        "background-color": "#21262d",
        "border-width": 1,
        "border-color": "#30363d"
      }
    },
    // Const nodes
    {
      selector: ".op-const",
      style: {
        "background-color": "#0d2818",
        "border-color": "#3fb950"
      }
    },
    // Arithmetic nodes
    {
      selector: ".op-arithmetic",
      style: {
        "background-color": "#0c2d6b",
        "border-color": "#58a6ff"
      }
    },
    // Compare nodes
    {
      selector: ".op-compare",
      style: {
        "background-color": "#2d1f00",
        "border-color": "#d29922"
      }
    },
    // Control flow nodes
    {
      selector: ".op-control",
      style: {
        "background-color": "#3d1015",
        "border-color": "#f85149"
      }
    },
    // Call nodes
    {
      selector: ".op-call",
      style: {
        "background-color": "#261748",
        "border-color": "#bc8cff"
      }
    },
    // Memory (Load/Store) nodes
    {
      selector: ".op-memory",
      style: {
        "background-color": "#1a2332",
        "border-color": "#79c0ff"
      }
    },
    // IO (Print/Return) nodes
    {
      selector: ".op-io",
      style: {
        "background-color": "#1c1c1c",
        "border-color": "#8b949e"
      }
    },
    // Selected node
    {
      selector: "node:selected",
      style: {
        "border-color": "#f0f6fc",
        "border-width": 2
      }
    },
    // Edges — base style
    {
      selector: "edge",
      style: {
        "width": 1.5,
        "line-color": "#484f58",
        "target-arrow-color": "#484f58",
        "target-arrow-shape": "triangle",
        "curve-style": "bezier",
        "label": "data(label)",
        "font-size": "9px",
        "color": "#6e7681",
        "text-rotation": "autorotate",
        "text-margin-y": -8,
        "arrow-scale": 0.8
      }
    },
    // TrueBlock edges — green
    {
      selector: 'edge[edgeType="TrueBlock"]',
      style: {
        "line-color": "#3fb950",
        "target-arrow-color": "#3fb950",
        "line-style": "dashed"
      }
    },
    // FalseBlock edges — red
    {
      selector: 'edge[edgeType="FalseBlock"]',
      style: {
        "line-color": "#f85149",
        "target-arrow-color": "#f85149",
        "line-style": "dashed"
      }
    }
  ];

  // --- Initialize Cytoscape ---
  function initCytoscape() {
    cy = cytoscape({
      container: document.getElementById("graph-container"),
      style: cytoscapeStyle,
      elements: [],
      layout: { name: "preset" },
      wheelSensitivity: 0.3
    });

    cy.on("tap", "node[nodeType='op']", function (evt) {
      showInspector(evt.target.data());
    });

    cy.on("tap", function (evt) {
      if (evt.target === cy) {
        clearInspector();
      }
    });
  }

  // --- Update graph ---
  function updateGraph(data) {
    if (!cy) return;

    var errorPanel = document.getElementById("error-panel");

    // Show errors if present
    if (data.errors && data.errors.length > 0) {
      errorPanel.classList.remove("hidden");
      errorPanel.innerHTML = data.errors
        .map(function (e) { return '<div class="error-item">' + escapeHtml(e) + '</div>'; })
        .join("");
      cy.elements().remove();
      return;
    }

    errorPanel.classList.add("hidden");
    errorPanel.innerHTML = "";

    // Build elements array
    var elements = [];
    if (data.nodes) {
      data.nodes.forEach(function (n) {
        var el = { group: "nodes", data: n.data };
        if (n.classes) el.classes = n.classes;
        elements.push(el);
      });
    }
    if (data.edges) {
      data.edges.forEach(function (e) {
        elements.push({ group: "edges", data: e.data });
      });
    }

    cy.elements().remove();
    cy.add(elements);
    cy.layout({
      name: "dagre",
      rankDir: "TB",
      nodeSep: 30,
      rankSep: 50,
      animate: true,
      animationDuration: 300
    }).run();
  }

  // --- Inspector ---
  function showInspector(data) {
    var content = document.getElementById("inspector-content");
    var fields = [
      { label: "ID", value: data.id },
      { label: "Op Type", value: data.opType },
      { label: "Result Type", value: data.resultType },
      { label: "Function", value: data.function },
      { label: "Block", value: data.block }
    ];

    content.innerHTML = fields
      .filter(function (f) { return f.value; })
      .map(function (f) {
        return '<div class="inspector-field">' +
          '<div class="label">' + escapeHtml(f.label) + '</div>' +
          '<div class="value">' + escapeHtml(f.value) + '</div>' +
          '</div>';
      })
      .join("");
  }

  function clearInspector() {
    document.getElementById("inspector-content").innerHTML =
      '<p class="placeholder">Click a node to inspect it.</p>';
  }

  // --- WebSocket ---
  function connectWebSocket() {
    setStatus("connecting");

    var protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    ws = new WebSocket(protocol + "//" + window.location.host + "/ws");

    ws.onopen = function () {
      setStatus("connected");
      if (reconnectTimer) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
      }
    };

    ws.onmessage = function (evt) {
      try {
        var msg = JSON.parse(evt.data);
        if (msg.type === "graph_update" && msg.data) {
          updateGraph(msg.data);
        }
      } catch (e) {
        console.error("Failed to parse WebSocket message:", e);
      }
    };

    ws.onclose = function () {
      setStatus("disconnected");
      scheduleReconnect();
    };

    ws.onerror = function () {
      setStatus("disconnected");
      ws.close();
    };
  }

  function scheduleReconnect() {
    if (reconnectTimer) return;
    reconnectTimer = setTimeout(function () {
      reconnectTimer = null;
      connectWebSocket();
    }, 2000);
  }

  function setStatus(state) {
    var el = document.getElementById("status");
    el.className = "status " + state;
    el.textContent = state;
  }

  // --- Initial load ---
  function loadInitialGraph() {
    fetch("/api/graph")
      .then(function (res) { return res.json(); })
      .then(function (data) { updateGraph(data); })
      .catch(function (err) { console.error("Failed to load initial graph:", err); });
  }

  // --- Utils ---
  function escapeHtml(str) {
    var div = document.createElement("div");
    div.appendChild(document.createTextNode(str));
    return div.innerHTML;
  }

  // --- Boot ---
  document.addEventListener("DOMContentLoaded", function () {
    initCytoscape();
    loadInitialGraph();
    connectWebSocket();
  });
})();
