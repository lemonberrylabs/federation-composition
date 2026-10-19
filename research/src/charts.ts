import guildTs from "../data/01-baseline-guild-ts.json";
import mitata from "../data/01-baseline-mitata.json";
import speedRust from "../data/03-speed-guild-rust.json";
import speedTs from "../data/03-speed-guild-ts.json";
import speedWasm from "../data/03-speed-guild-wasm.json";
import optRust from "../data/04-optimized-guild-rust.json";
import optTs from "../data/04-optimized-guild-ts.json";
import optWasm from "../data/04-optimized-guild-wasm.json";

// Each chart renders into a placeholder `<div data-chart="<name>">` in a journal entry.
type Chart = (root: HTMLElement) => void;

const SVG_NS = "http://www.w3.org/2000/svg";

function svg(
  tag: string,
  attrs: Record<string, string | number>,
  text?: string,
) {
  const node = document.createElementNS(SVG_NS, tag);
  for (const [key, value] of Object.entries(attrs)) {
    node.setAttribute(key, String(value));
  }
  if (text !== undefined) {
    node.textContent = text;
  }
  return node;
}

function html(tag: string, className: string, text?: string) {
  const node = document.createElement(tag);
  node.className = className;
  if (text !== undefined) {
    node.textContent = text;
  }
  return node;
}

const count = new Intl.NumberFormat("en-US");

export function formatMs(ms: number) {
  if (ms >= 1000) {
    return `${(ms / 1000).toFixed(2)} s`;
  }
  if (ms >= 100) {
    return `${ms.toFixed(0)} ms`;
  }
  if (ms >= 10) {
    return `${ms.toFixed(1)} ms`;
  }
  return `${ms.toFixed(2)} ms`;
}

// A horizontal bar with a 4px rounded data end and a square baseline.
function barPath(x: number, y: number, width: number, height: number) {
  const r = Math.min(4, width, height / 2);
  return `M${x},${y}h${width - r}a${r},${r} 0 0 1 ${r},${r}v${height - 2 * r}a${r},${r} 0 0 1 -${r},${r}h-${width - r}z`;
}

// A vertical column with a 4px rounded top and a square baseline.
function columnPath(
  x: number,
  baseline: number,
  width: number,
  height: number,
) {
  const r = Math.min(4, height, width / 2);
  const top = baseline - height;
  return `M${x},${baseline}V${top + r}a${r},${r} 0 0 1 ${r},-${r}h${width - 2 * r}a${r},${r} 0 0 1 ${r},${r}V${baseline}z`;
}

function legend(items: Array<{ label: string; series: number }>) {
  const root = html("div", "legend");
  for (const item of items) {
    const entry = html("span", "legend-item");
    entry.append(html("span", `swatch series-${item.series}`), item.label);
    root.append(entry);
  }
  return root;
}

function median(values: number[]) {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}

type Tile = { label: string; value: string; note: string };

function renderTiles(root: HTMLElement, tiles: Tile[]) {
  const grid = html("div", "tiles");
  for (const tile of tiles) {
    const node = html("div", "tile");
    node.append(
      html("div", "tile-label", tile.label),
      html("div", "tile-value", tile.value),
      html("div", "tile-note", tile.note),
    );
    grid.append(node);
  }
  root.append(grid);
}

const baselineTiles: Chart = (root) => {
  const huge = guildTs.fixtures.find((f) => f.id === "huge-schema")!;
  const dgs = guildTs.fixtures.find((f) => f.id === "dgs")!;
  const tiles = [
    {
      label: "Tests passing",
      value: count.format(7343),
      note: "25 skipped, 8 todo",
    },
    {
      label: "Huge schema",
      value: formatMs(huge.medianMs),
      note: `${huge.subgraphs} subgraphs, median`,
    },
    {
      label: "Medium schema",
      value: formatMs(dgs.medianMs),
      note: `${dgs.subgraphs} subgraphs, median`,
    },
    {
      label: "Typical test composition",
      value: formatMs(median(guildTs.cases.map((c) => c[0]))),
      note: `median of ${count.format(guildTs.cases.length)}`,
    },
  ];
  renderTiles(root, tiles);
};

const portTiles: Chart = (root) =>
  renderTiles(root, [
    {
      label: "Tests passing on WebAssembly",
      value: count.format(7343),
      note: "unchanged TypeScript suite",
    },
    {
      label: "Compositions identical",
      value: "3,564 / 3,564",
      note: "native Rust vs TypeScript",
    },
    {
      label: "Rust written",
      value: "47.5K lines",
      note: "graphql, composition, bindings",
    },
    {
      label: "Wall-clock time",
      value: "≈ 2 h",
      note: "graphql port to passing suite",
    },
  ]);

type BarSeries = { name: string; series: number };
type BarGroup = { title: string; note: string; values: number[] };

// Small multiples of horizontal bars, one panel per group, each with its own scale:
// the fixtures span three orders of magnitude, so a shared axis would flatten the small ones.
function barMultiples(
  root: HTMLElement,
  title: string,
  subtitle: string,
  series: BarSeries[],
  groups: BarGroup[],
) {
  root.append(
    html("div", "chart-title", title),
    html("div", "chart-subtitle", subtitle),
    legend(series.map((s) => ({ label: s.name, series: s.series }))),
  );
  const grid = html("div", "multiples");
  root.append(grid);

  for (const group of groups) {
    const panel = html("div", "multiple");
    panel.append(
      html("div", "multiple-title", group.title),
      html("div", "multiple-note", group.note),
    );
    const width = 260;
    const valueWidth = 64;
    const barHeight = 20;
    const gap = 8;
    const height = series.length * barHeight + (series.length - 1) * gap;
    const plot = width - valueWidth;
    const max = Math.max(...group.values);
    const chart = svg("svg", {
      viewBox: `0 0 ${width} ${height}`,
      width: "100%",
      role: "img",
    });
    chart.setAttribute(
      "aria-label",
      `${group.title}: ${series.map((s, i) => `${s.name} ${formatMs(group.values[i])}`).join(", ")}`,
    );
    series.forEach((bar, i) => {
      const value = group.values[i];
      const y = i * (barHeight + gap);
      const w = Math.max(2, (value / max) * plot);
      const g = svg("g", {
        "data-tip": `${group.title}\n${bar.name}: ${formatMs(value)}`,
        tabindex: 0,
      });
      g.append(
        svg("rect", {
          x: 0,
          y: y - gap / 2,
          width,
          height: barHeight + gap,
          class: "hit",
        }),
        svg("path", {
          d: barPath(0, y, w, barHeight),
          class: `mark series-${bar.series}`,
        }),
        svg(
          "text",
          {
            x: w + 6,
            y: y + barHeight / 2,
            class: "value",
            "dominant-baseline": "central",
          },
          formatMs(value),
        ),
      );
      chart.append(g);
    });
    chart.append(
      svg("line", { x1: 0.5, x2: 0.5, y1: 0, y2: height, class: "axis" }),
    );
    panel.append(chart);
    grid.append(panel);
  }
}

const baselineMitata: Chart = (root) =>
  barMultiples(
    root,
    "Apollo vs this library on the benchmark fixtures",
    "Average time per iteration, pnpm bench (mitata). Each panel has its own scale.",
    [
      { name: "Apollo", series: 2 },
      { name: "This library (TypeScript)", series: 1 },
    ],
    mitata.groups.map((g) => ({
      title: g.id,
      note: `${g.subgraphs} subgraphs · ${(g.apolloMs / g.guildMs).toFixed(1)}× faster than Apollo`,
      values: [g.apolloMs, g.guildMs],
    })),
  );

const IMPLEMENTATIONS: BarSeries[] = [
  { name: "TypeScript", series: 1 },
  { name: "WebAssembly", series: 3 },
  { name: "Native Rust", series: 7 },
];

type RunData = Pick<typeof speedTs, "fixtures" | "cases">;
// One benchmark session: the three implementations run one after another on the same corpus.
type Session = { ts: RunData; wasm: RunData; rust: RunData };
const SPEED: Session = { ts: speedTs, wasm: speedWasm, rust: speedRust };
const OPTIMIZED: Session = { ts: optTs, wasm: optWasm, rust: optRust };

const fixtureBars =
  (session: Session): Chart =>
  (root) =>
    barMultiples(
      root,
      "The benchmark fixtures in each implementation",
      "Median composeServices() time. WebAssembly includes converting the AST across the boundary. Each panel has its own scale.",
      IMPLEMENTATIONS,
      session.ts.fixtures.map((f, i) => {
        const values = [
          f.medianMs,
          session.wasm.fixtures[i].medianMs,
          session.rust.fixtures[i].medianMs,
        ];
        return {
          title: f.id,
          note: `${f.subgraphs} subgraphs · native ${(values[0] / values[2]).toFixed(2)}× faster`,
          values,
        };
      }),
    );

// Distribution of per-composition speedups over TypeScript, one panel per implementation,
// on shared log-spaced bins so the panels compare directly. `edges` are the bin edges, in ×.
const speedupHistogram =
  (session: Session, edges: number[]): Chart =>
  (root) => {
    root.append(
      html(
        "div",
        "chart-title",
        "Speedup over TypeScript, per test-suite composition",
      ),
      html(
        "div",
        "chart-subtitle",
        `TypeScript median time divided by the other implementation's, for each of the ${count.format(session.ts.cases.length)} compositions. Left of 1× means slower than TypeScript.`,
      ),
    );
    const grid = html("div", "multiples");
    root.append(grid);
    const panels = [
      { name: "WebAssembly", series: 3, data: session.wasm },
      { name: "Native Rust", series: 7, data: session.rust },
    ];
    const binned = panels.map((p) => {
      const bins = edges
        .slice(0, -1)
        .map((lo, i) => ({ lo, hi: edges[i + 1], n: 0 }));
      session.ts.cases.forEach((c, i) => {
        const ratio = c[0] / p.data.cases[i][0];
        const bin =
          bins.find((b) => ratio >= b.lo && ratio < b.hi) ??
          (ratio < edges[0] ? bins[0] : bins[bins.length - 1]);
        bin.n++;
      });
      return bins;
    });
    const max = Math.max(...binned.flat().map((b) => b.n));
    const yMax = Math.ceil(max / 500) * 500;

    panels.forEach((p, pi) => {
      const panel = html("div", "multiple");
      const ratios = session.ts.cases.map((c, i) => c[0] / p.data.cases[i][0]);
      const geomean = Math.exp(
        ratios.reduce((s, r) => s + Math.log(r), 0) / ratios.length,
      );
      const slower = ratios.filter((r) => r < 1).length;
      panel.append(
        html("div", "multiple-title", p.name),
        html(
          "div",
          "multiple-note",
          `geometric mean ${geomean.toFixed(2)}× · ${slower === 0 ? "none" : count.format(slower)} slower than TypeScript`,
        ),
      );
      const width = 300;
      const height = 170;
      const margin = { top: 8, right: 4, bottom: 24, left: 36 };
      const plotW = width - margin.left - margin.right;
      const plotH = height - margin.top - margin.bottom;
      const bins = binned[pi];
      const band = plotW / bins.length;
      const colW = Math.min(24, band - 2);
      const baseline = margin.top + plotH;
      const chart = svg("svg", {
        viewBox: `0 0 ${width} ${height}`,
        width: "100%",
        role: "img",
      });
      chart.setAttribute(
        "aria-label",
        `${p.name} speedup histogram; geometric mean ${geomean.toFixed(2)}×`,
      );
      for (let t = 0; t <= yMax; t += 500) {
        const y = baseline - (t / yMax) * plotH;
        chart.append(
          svg("line", {
            x1: margin.left,
            x2: width - margin.right,
            y1: y,
            y2: y,
            class: t === 0 ? "axis" : "grid",
          }),
          svg(
            "text",
            {
              x: margin.left - 6,
              y,
              class: "tick",
              "text-anchor": "end",
              "dominant-baseline": "central",
            },
            t >= 1000 ? `${t / 1000}K` : String(t),
          ),
        );
      }
      bins.forEach((bin, i) => {
        const x = margin.left + i * band;
        const h = (bin.n / yMax) * plotH;
        const g = svg("g", {
          "data-tip": `${p.name}: ${bin.lo}× – ${bin.hi}×\n${count.format(bin.n)} compositions`,
          tabindex: 0,
        });
        g.append(
          svg("rect", {
            x,
            y: margin.top,
            width: band,
            height: plotH,
            class: "hit",
          }),
        );
        if (bin.n > 0) {
          g.append(
            svg("path", {
              d: columnPath(
                x + (band - colW) / 2,
                baseline,
                colW,
                Math.max(h, 2),
              ),
              class: `mark series-${p.series}`,
            }),
          );
        }
        chart.append(g);
      });
      edges
        .filter((_, i) => i % 2 === 0)
        .forEach((v) => {
          const x = margin.left + edges.indexOf(v) * band;
          chart.append(
            svg("line", {
              x1: x,
              x2: x,
              y1: baseline,
              y2: baseline + 4,
              class: "axis",
            }),
            svg(
              "text",
              { x, y: baseline + 16, class: "tick", "text-anchor": "middle" },
              `${v}×`,
            ),
          );
        });
      panel.append(chart);
      grid.append(panel);
    });
  };

const geo = (r: number[]) =>
  Math.exp(r.reduce((s, x) => s + Math.log(x), 0) / r.length);
const hugeMs = (d: RunData) =>
  d.fixtures.find((f) => f.id === "huge-schema")!.medianMs;
const speedups = (base: RunData, other: RunData) =>
  base.cases.map((c, i) => c[0] / other.cases[i][0]);

const speedTiles: Chart = (root) =>
  renderTiles(root, [
    {
      label: "Native, test compositions",
      value: `${geo(speedups(speedTs, speedRust)).toFixed(2)}×`,
      note: "geometric mean speedup",
    },
    {
      label: "WebAssembly, test compositions",
      value: `${geo(speedups(speedTs, speedWasm)).toFixed(2)}×`,
      note: "geometric mean speedup",
    },
    {
      label: "Native, huge schema",
      value: `${(hugeMs(speedTs) / hugeMs(speedRust)).toFixed(2)}×`,
      note: `${formatMs(hugeMs(speedRust))} vs ${formatMs(hugeMs(speedTs))}`,
    },
    {
      label: "WebAssembly, huge schema",
      value: `${(hugeMs(speedTs) / hugeMs(speedWasm)).toFixed(2)}×`,
      note: `${formatMs(hugeMs(speedWasm))} vs ${formatMs(hugeMs(speedTs))}`,
    },
  ]);

// Speedups over TypeScript after optimization, each with the faithful port's value for comparison.
const optimizedTiles: Chart = (root) =>
  renderTiles(root, [
    {
      label: "Native, test compositions",
      value: `${geo(speedups(optTs, optRust)).toFixed(2)}×`,
      note: `geometric mean · was ${geo(speedups(speedTs, speedRust)).toFixed(2)}×`,
    },
    {
      label: "WebAssembly, test compositions",
      value: `${geo(speedups(optTs, optWasm)).toFixed(2)}×`,
      note: `geometric mean · was ${geo(speedups(speedTs, speedWasm)).toFixed(2)}×`,
    },
    {
      label: "Native, huge schema",
      value: `${(hugeMs(optTs) / hugeMs(optRust)).toFixed(2)}×`,
      note: `${formatMs(hugeMs(optRust))} · was ${formatMs(hugeMs(speedRust))}`,
    },
    {
      label: "WebAssembly, huge schema",
      value: `${(hugeMs(optTs) / hugeMs(optWasm)).toFixed(2)}×`,
      note: `${formatMs(hugeMs(optWasm))} · was ${formatMs(hugeMs(speedWasm))}`,
    },
  ]);

// Histogram of per-composition medians on log-spaced bins, 4 per decade.
const baselineHistogram: Chart = (root) => {
  root.append(
    html("div", "chart-title", "How long each test-suite composition takes"),
    html(
      "div",
      "chart-subtitle",
      `Median time of each of the ${count.format(guildTs.cases.length)} unique compositions recorded from the test suite. Log-scale bins.`,
    ),
  );
  const binsPerDecade = 4;
  const lowExp = -1;
  const highExp = 2;
  const edges: number[] = [];
  for (let k = lowExp * binsPerDecade; k <= highExp * binsPerDecade; k++) {
    edges.push(10 ** (k / binsPerDecade));
  }
  const bins = edges
    .slice(0, -1)
    .map((lo, i) => ({ lo, hi: edges[i + 1], n: 0 }));
  for (const [ms] of guildTs.cases) {
    const bin =
      bins.find((b) => ms >= b.lo && ms < b.hi) ??
      (ms < edges[0] ? bins[0] : bins[bins.length - 1]);
    bin.n++;
  }

  const container = html("div", "chart-body");
  root.append(container);

  const draw = () => {
    container.replaceChildren();
    const width = Math.max(300, container.clientWidth);
    const height = 220;
    const margin = { top: 12, right: 8, bottom: 28, left: 44 };
    const plotW = width - margin.left - margin.right;
    const plotH = height - margin.top - margin.bottom;
    const max = Math.max(...bins.map((b) => b.n));
    const tickStep = max > 1000 ? 500 : max > 200 ? 100 : 20;
    const yMax = Math.ceil(max / tickStep) * tickStep;
    const band = plotW / bins.length;
    const colW = Math.min(24, band - 2);
    const baseline = margin.top + plotH;

    const chart = svg("svg", {
      viewBox: `0 0 ${width} ${height}`,
      width,
      height,
      role: "img",
    });
    chart.setAttribute(
      "aria-label",
      "Histogram of composition times; see the table below for values.",
    );

    for (let t = 0; t <= yMax; t += tickStep) {
      const y = baseline - (t / yMax) * plotH;
      chart.append(
        svg("line", {
          x1: margin.left,
          x2: width - margin.right,
          y1: y,
          y2: y,
          class: t === 0 ? "axis" : "grid",
        }),
        svg(
          "text",
          {
            x: margin.left - 6,
            y,
            class: "tick",
            "text-anchor": "end",
            "dominant-baseline": "central",
          },
          t >= 1000 ? `${t / 1000}K` : String(t),
        ),
      );
    }

    bins.forEach((bin, i) => {
      const x = margin.left + i * band;
      const h = (bin.n / yMax) * plotH;
      const g = svg("g", {
        "data-tip": `${formatMs(bin.lo)} – ${formatMs(bin.hi)}\n${count.format(bin.n)} compositions`,
        tabindex: 0,
      });
      g.append(
        svg("rect", {
          x,
          y: margin.top,
          width: band,
          height: plotH,
          class: "hit",
        }),
      );
      if (bin.n > 0) {
        g.append(
          svg("path", {
            d: columnPath(
              x + (band - colW) / 2,
              baseline,
              colW,
              Math.max(h, 2),
            ),
            class: "mark series-1",
          }),
        );
      }
      chart.append(g);
    });

    for (let e = lowExp; e <= highExp; e++) {
      const x = margin.left + (e - lowExp) * binsPerDecade * band;
      chart.append(
        svg("line", {
          x1: x,
          x2: x,
          y1: baseline,
          y2: baseline + 4,
          class: "axis",
        }),
        svg(
          "text",
          { x, y: baseline + 18, class: "tick", "text-anchor": "middle" },
          `${10 ** e} ms`,
        ),
      );
    }
    container.append(chart);
  };

  draw();
  new ResizeObserver(draw).observe(container);
};

const charts: Record<string, Chart> = {
  "baseline-tiles": baselineTiles,
  "port-tiles": portTiles,
  "baseline-mitata": baselineMitata,
  "baseline-histogram": baselineHistogram,
  "speed-tiles": speedTiles,
  "speed-fixtures": fixtureBars(SPEED),
  "speedup-histogram": speedupHistogram(
    SPEED,
    [0.5, 0.71, 1, 1.41, 2, 2.83, 4, 5.66, 8],
  ),
  "optimized-tiles": optimizedTiles,
  "optimized-fixtures": fixtureBars(OPTIMIZED),
  "optimized-histogram": speedupHistogram(
    OPTIMIZED,
    [0.5, 0.71, 1, 1.41, 2, 2.83, 4, 5.66, 8, 11.3, 16, 22.6, 32],
  ),
};

export function renderCharts(scope: HTMLElement) {
  for (const root of scope.querySelectorAll<HTMLElement>("[data-chart]")) {
    const chart = charts[root.dataset.chart!];
    if (!chart) {
      throw new Error(`Unknown chart "${root.dataset.chart}"`);
    }
    root.classList.add("chart");
    chart(root);
  }
}
