import guildTs from "../data/01-baseline-guild-ts.json";
import mitata from "../data/01-baseline-mitata.json";

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

// Small multiples, one per fixture, each with its own scale: the fixtures span
// three orders of magnitude, so a shared axis would flatten the small ones.
const baselineMitata: Chart = (root) => {
  root.append(
    html(
      "div",
      "chart-title",
      "Apollo vs this library on the benchmark fixtures",
    ),
    html(
      "div",
      "chart-subtitle",
      "Average time per iteration, pnpm bench (mitata). Each panel has its own scale.",
    ),
    legend([
      { label: "Apollo", series: 2 },
      { label: "This library (TypeScript)", series: 1 },
    ]),
  );
  const grid = html("div", "multiples");
  root.append(grid);

  for (const group of mitata.groups) {
    const panel = html("div", "multiple");
    panel.append(
      html("div", "multiple-title", group.id),
      html(
        "div",
        "multiple-note",
        `${group.subgraphs} subgraphs · ${(group.apolloMs / group.guildMs).toFixed(1)}× faster than Apollo`,
      ),
    );
    const width = 260;
    const valueWidth = 64;
    const barHeight = 20;
    const gap = 8;
    const height = barHeight * 2 + gap;
    const plot = width - valueWidth;
    const max = Math.max(group.apolloMs, group.guildMs);
    const chart = svg("svg", {
      viewBox: `0 0 ${width} ${height}`,
      width: "100%",
      role: "img",
    });
    chart.setAttribute(
      "aria-label",
      `${group.id}: Apollo ${formatMs(group.apolloMs)}, this library ${formatMs(group.guildMs)}`,
    );
    const bars = [
      { name: "Apollo", value: group.apolloMs, series: 2 },
      { name: "This library", value: group.guildMs, series: 1 },
    ];
    bars.forEach((bar, i) => {
      const y = i * (barHeight + gap);
      const w = Math.max(2, (bar.value / max) * plot);
      const g = svg("g", {
        "data-tip": `${group.id}\n${bar.name}: ${formatMs(bar.value)}`,
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
          formatMs(bar.value),
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
};

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
