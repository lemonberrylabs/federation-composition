// Prints conformance/out/summary.json (written by `pnpm conformance`) as Markdown, for the
// workflow's job summary. Prints nothing if the run wrote no summary.
import fs from "node:fs";

const file = new URL("../../conformance/out/summary.json", import.meta.url);
if (fs.existsSync(file)) {
  const summary = JSON.parse(fs.readFileSync(file, "utf8"));
  const lines = [
    `### Conformance with upstream ${summary.ref} (${(summary.commit ?? "").slice(0, 12)})`,
    "",
  ];
  for (const [step, result] of Object.entries(summary.steps)) {
    lines.push(
      `- ${result.ok ? "ok" : "**FAIL**"} ${step}: ${result.description}`,
    );
  }
  console.log(lines.join("\n"));
}
