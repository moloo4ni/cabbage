<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte';
  import { api } from './api';
  import type { GraphData } from './api';
  import { RotateCcw } from '@lucide/svelte';

  export let activeNotePath: string | null = null;

  const dispatch = createEventDispatcher<{ navigate: string }>();

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D | null = null;
  let width = 0;
  let height = 0;
  let animFrame = 0;
  let simRunning = false;
  let errorMsg = '';

  // ── Simulation state ────────────────────────────────────────────────────────

  interface SimNode {
    id: string;
    x: number;
    y: number;
    vx: number;
    vy: number;
  }

  interface SimEdge {
    si: number; // index into nodes array
    ti: number;
    sourceId: string;
    targetId: string;
  }

  let nodes: SimNode[] = [];
  let edges: SimEdge[] = [];

  // ── Pan / zoom state ────────────────────────────────────────────────────────

  let panX = 0;
  let panY = 0;
  let scale = 1;
  let isPanning = false;
  let panStartX = 0;
  let panStartY = 0;
  let panOriginX = 0;
  let panOriginY = 0;

  // ── Load and initialise ─────────────────────────────────────────────────────

  async function loadGraph() {
    try {
      errorMsg = '';
      const data: GraphData = await api.getGraph();
      initSimulation(data);
    } catch (e) {
      errorMsg = `Failed to load graph: ${e}`;
    }
  }

  function initSimulation(data: GraphData) {
    const n = data.nodes.length;
    const cx = width / 2;
    const cy = height / 2;
    const radius = Math.min(width, height) * 0.35;

    // Place nodes on a circle for a stable initial layout
    nodes = data.nodes.map((nd, i) => {
      const angle = (i / n) * Math.PI * 2;
      return {
        id: nd.id,
        x: cx + radius * Math.cos(angle),
        y: cy + radius * Math.sin(angle),
        vx: 0,
        vy: 0,
      };
    });

    // Build index map for O(1) lookup
    const idToIndex = new Map<string, number>(nodes.map((nd, i) => [nd.id, i]));

    edges = [];
    for (const e of data.edges) {
      const si = idToIndex.get(e.source);
      const ti = idToIndex.get(e.target);
      if (si !== undefined && ti !== undefined) {
        edges.push({ si, ti, sourceId: e.source, targetId: e.target });
      }
    }

    // Reset pan/zoom to center
    panX = 0;
    panY = 0;
    scale = 1;

    startSimulation();
  }

  // ── Force-directed simulation ───────────────────────────────────────────────

  const REPULSION    = 4000;
  const SPRING_LEN   = 120;
  const SPRING_K     = 0.04;
  const CENTER_STR   = 0.008;
  const DAMPING      = 0.82;
  const ALPHA        = 0.25;
  const MAX_TICKS    = 350;

  function startSimulation() {
    simRunning = true;
    let ticks = 0;

    function tick() {
      if (!simRunning || ticks >= MAX_TICKS) {
        simRunning = false;
        render();
        return;
      }
      simulate();
      render();
      ticks++;
      animFrame = requestAnimationFrame(tick);
    }

    animFrame = requestAnimationFrame(tick);
  }

  function simulate() {
    const n = nodes.length;
    if (n === 0) return;

    const cx = width / 2;
    const cy = height / 2;
    const fx = new Array<number>(n).fill(0);
    const fy = new Array<number>(n).fill(0);

    // Repulsion between every pair of nodes
    for (let i = 0; i < n; i++) {
      for (let j = i + 1; j < n; j++) {
        const dx = nodes[j].x - nodes[i].x;
        const dy = nodes[j].y - nodes[i].y;
        const d2 = dx * dx + dy * dy + 1;
        const d  = Math.sqrt(d2);
        const f  = REPULSION / d2;
        const nx = (dx / d) * f;
        const ny = (dy / d) * f;
        fx[i] -= nx;
        fy[i] -= ny;
        fx[j] += nx;
        fy[j] += ny;
      }
    }

    // Spring attraction along edges
    for (const e of edges) {
      const s = nodes[e.si];
      const t = nodes[e.ti];
      const dx = t.x - s.x;
      const dy = t.y - s.y;
      const d  = Math.sqrt(dx * dx + dy * dy) + 0.01;
      const f  = (d - SPRING_LEN) * SPRING_K;
      const nx = (dx / d) * f;
      const ny = (dy / d) * f;
      fx[e.si] += nx;
      fy[e.si] += ny;
      fx[e.ti] -= nx;
      fy[e.ti] -= ny;
    }

    // Integrate + centering + damping
    for (let i = 0; i < n; i++) {
      fx[i] += (cx - nodes[i].x) * CENTER_STR;
      fy[i] += (cy - nodes[i].y) * CENTER_STR;
      nodes[i].vx = (nodes[i].vx + fx[i] * ALPHA) * DAMPING;
      nodes[i].vy = (nodes[i].vy + fy[i] * ALPHA) * DAMPING;
      nodes[i].x += nodes[i].vx;
      nodes[i].y += nodes[i].vy;
    }
  }

  // ── Rendering ───────────────────────────────────────────────────────────────

  // Derive the display-friendly active note name from the vault-relative path
  function activeId(): string | null {
    if (!activeNotePath) return null;
    return activeNotePath.replace(/\.md$/i, '').split('/').pop() ?? null;
  }

  function render() {
    if (!ctx || width === 0 || height === 0) return;
    ctx.clearRect(0, 0, width, height);

    ctx.save();
    ctx.translate(panX, panY);
    ctx.scale(scale, scale);

    // ── Edges ──────────────────────────────────────────────────────────────
    ctx.strokeStyle = '#d1d5db';
    ctx.lineWidth = 1 / scale;

    for (const e of edges) {
      const s = nodes[e.si];
      const t = nodes[e.ti];
      ctx.beginPath();
      ctx.moveTo(s.x, s.y);
      ctx.lineTo(t.x, t.y);
      ctx.stroke();
    }

    // ── Nodes ──────────────────────────────────────────────────────────────
    const current = activeId();
    ctx.textAlign = 'center';
    ctx.textBaseline = 'top';

    for (const nd of nodes) {
      const isActive = nd.id === current;
      const r = isActive ? 9 : 6;

      // Node circle
      ctx.beginPath();
      ctx.arc(nd.x, nd.y, r, 0, Math.PI * 2);
      ctx.fillStyle = isActive ? '#10b981' : '#9ca3af';
      ctx.fill();

      if (isActive) {
        ctx.strokeStyle = '#065f46';
        ctx.lineWidth = 1.5 / scale;
        ctx.stroke();
      }

      // Label
      const fontSize = Math.max(10, 11 / scale);
      ctx.font = `${fontSize}px system-ui, sans-serif`;
      ctx.fillStyle = '#374151';
      ctx.fillText(nd.id, nd.x, nd.y + r + 4 / scale);
    }

    ctx.restore();
  }

  // ── Interaction ─────────────────────────────────────────────────────────────

  function toGraphCoords(mx: number, my: number): [number, number] {
    return [(mx - panX) / scale, (my - panY) / scale];
  }

  function hitTest(mx: number, my: number): SimNode | null {
    const [gx, gy] = toGraphCoords(mx, my);
    const hitR = 10 / scale;
    for (const nd of nodes) {
      const dx = nd.x - gx;
      const dy = nd.y - gy;
      if (dx * dx + dy * dy <= hitR * hitR) return nd;
    }
    return null;
  }

  function handleClick(e: MouseEvent) {
    const rect = canvas.getBoundingClientRect();
    const node = hitTest(e.clientX - rect.left, e.clientY - rect.top);
    if (node) {
      dispatch('navigate', node.id);
    }
  }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const factor = e.deltaY < 0 ? 1.12 : 1 / 1.12;
    // Zoom toward cursor
    panX = mx - (mx - panX) * factor;
    panY = my - (my - panY) * factor;
    scale *= factor;
    scale = Math.max(0.1, Math.min(scale, 10));
    if (!simRunning) render();
  }

  function handleMouseDown(e: MouseEvent) {
    if (e.button !== 0) return;
    const rect = canvas.getBoundingClientRect();
    if (hitTest(e.clientX - rect.left, e.clientY - rect.top)) return;
    isPanning = true;
    panStartX = e.clientX;
    panStartY = e.clientY;
    panOriginX = panX;
    panOriginY = panY;
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isPanning) return;
    panX = panOriginX + e.clientX - panStartX;
    panY = panOriginY + e.clientY - panStartY;
    if (!simRunning) render();
  }

  function handleMouseUp() {
    isPanning = false;
  }

  // ── Lifecycle ───────────────────────────────────────────────────────────────

  onMount(() => {
    ctx = canvas.getContext('2d');
    const ro = new ResizeObserver(() => {
      const rect = canvas.parentElement!.getBoundingClientRect();
      width = rect.width;
      height = rect.height;
      canvas.width = width;
      canvas.height = height;
      if (nodes.length === 0) {
        loadGraph();
      } else {
        if (!simRunning) render();
      }
    });
    ro.observe(canvas.parentElement!);
    return () => ro.disconnect();
  });

  onDestroy(() => {
    simRunning = false;
    if (animFrame) cancelAnimationFrame(animFrame);
  });

  // Re-render when active note changes (highlight update only, no re-simulation)
  $: if (ctx && !simRunning && nodes.length > 0) {
    render();
  }
  $: void activeNotePath, (() => { if (ctx && !simRunning) render(); })();
</script>

<div class="graph-container">
  {#if errorMsg}
    <div class="graph-error">{errorMsg}</div>
  {/if}

  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <canvas
    bind:this={canvas}
    on:click={handleClick}
    on:wheel={handleWheel}
    on:mousedown={handleMouseDown}
    on:mousemove={handleMouseMove}
    on:mouseup={handleMouseUp}
    on:mouseleave={handleMouseUp}
  ></canvas>

  <div class="graph-hint">
    Scroll to zoom · Drag to pan · Click a node to open it
  </div>

  <button class="refresh-btn" title="Reload graph" on:click={loadGraph}>
    <RotateCcw size={14} />
  </button>
</div>

<style>
  .graph-container {
    position: relative;
    width: 100%;
    height: 100%;
    background: var(--bg-color);
    overflow: hidden;
  }

  canvas {
    display: block;
    width: 100%;
    height: 100%;
    cursor: grab;
  }

  canvas:active {
    cursor: grabbing;
  }

  .graph-error {
    position: absolute;
    top: 10px;
    left: 50%;
    transform: translateX(-50%);
    background: #fee2e2;
    color: #991b1b;
    font-size: 13px;
    padding: 6px 14px;
    border-radius: 4px;
    z-index: 10;
    white-space: nowrap;
  }

  .graph-hint {
    position: absolute;
    bottom: 12px;
    left: 50%;
    transform: translateX(-50%);
    font-size: 11px;
    color: var(--text-muted);
    pointer-events: none;
    white-space: nowrap;
  }

  .refresh-btn {
    position: absolute;
    top: 10px;
    right: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 6px;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    background: var(--sidebar-bg);
    color: var(--text-muted);
    cursor: pointer;
  }

  .refresh-btn:hover {
    color: var(--text-main);
    border-color: #9ca3af;
  }
</style>
