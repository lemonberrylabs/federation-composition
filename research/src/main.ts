import { marked } from "marked";
import { renderCharts } from "./charts";
import { entries, phases } from "./journal";
import { installTooltip } from "./tooltip";
import "./style.css";

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className?: string,
  text?: string,
) {
  const node = document.createElement(tag);
  if (className) {
    node.className = className;
  }
  if (text !== undefined) {
    node.textContent = text;
  }
  return node;
}

const app = document.getElementById("app")!;

const header = el("header", "site-header");
header.append(
  el("p", "eyebrow", "Research journal"),
  el("h1", undefined, "Rewriting federation composition in Rust"),
  el(
    "p",
    "lede",
    "A port of @theguild/federation-composition from TypeScript to Rust, checked against the existing test suite and measured at each milestone.",
  ),
);

const roadmap = el("ol", "roadmap");
for (const phase of phases) {
  const item = el("li", `phase phase-${phase.status.replace(" ", "-")}`);
  item.append(
    el("span", "phase-title", phase.title),
    el("span", "phase-status", phase.status),
  );
  roadmap.append(item);
}
header.append(roadmap);

const toc = el("nav", "toc");
toc.setAttribute("aria-label", "Milestones");
toc.append(el("h2", undefined, "Milestones"));
const tocList = el("ol");
for (const entry of entries) {
  const item = el("li");
  const link = el("a", undefined, entry.title);
  link.href = `#${entry.slug}`;
  item.append(
    el("span", "toc-date", entry.date),
    link,
    el("p", "toc-summary", entry.summary),
  );
  tocList.append(item);
}
toc.append(tocList);

const main = el("main");
for (const entry of entries) {
  const article = el("article", "entry");
  article.id = entry.slug;
  const meta = el(
    "p",
    "entry-meta",
    `Milestone ${entry.milestone} · ${entry.date}`,
  );
  const body = el("div", "entry-body");
  body.innerHTML = marked.parse(entry.body, { async: false });
  for (const table of body.querySelectorAll("table")) {
    const wrapper = el("div", "table-scroll");
    table.replaceWith(wrapper);
    wrapper.append(table);
  }
  article.append(meta, el("h2", "entry-title", entry.title), body);
  main.append(article);
}

app.append(header, toc, main);
renderCharts(main);
installTooltip();
