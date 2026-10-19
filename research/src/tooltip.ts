// One shared tooltip for every chart. Marks opt in with a data-tip attribute;
// "\n" in the value separates lines.
const tooltip = document.getElementById("tooltip")!;

function show(target: Element, x: number, y: number) {
  const tip = target.getAttribute("data-tip");
  if (!tip) {
    return;
  }
  tooltip.replaceChildren(
    ...tip.split("\n").map((line, i) => {
      const div = document.createElement("div");
      div.textContent = line;
      if (i === 0) {
        div.className = "tooltip-title";
      }
      return div;
    }),
  );
  tooltip.hidden = false;
  const { width, height } = tooltip.getBoundingClientRect();
  const left = Math.min(x + 12, window.innerWidth - width - 8);
  const top = y - height - 12 < 8 ? y + 16 : y - height - 12;
  tooltip.style.transform = `translate(${Math.max(8, left)}px, ${top}px)`;
}

export function installTooltip() {
  document.addEventListener("pointermove", (event) => {
    const target = (event.target as Element | null)?.closest("[data-tip]");
    if (target) {
      show(target, event.clientX, event.clientY);
    } else {
      tooltip.hidden = true;
    }
  });
  document.addEventListener("focusin", (event) => {
    const target = (event.target as Element).closest("[data-tip]");
    if (target) {
      const rect = target.getBoundingClientRect();
      show(target, rect.left + rect.width / 2, rect.top);
    }
  });
  document.addEventListener("focusout", () => {
    tooltip.hidden = true;
  });
}
