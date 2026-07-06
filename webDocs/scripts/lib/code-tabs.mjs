/**
 * Shared HTML for easy / typed Nyra code tab panels.
 */
import fs from "node:fs";
import path from "node:path";

export function escapeHtml(s) {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

export function readPair(root, relPlain) {
  const plainPath = path.join(root, relPlain);
  const typedPath = path.join(
    root,
    relPlain.replace(/\.ny$/, ".typed.ny"),
  );
  if (!fs.existsSync(plainPath) || !fs.existsSync(typedPath)) {
    return null;
  }
  return {
    rel: relPlain.replace(/\\/g, "/"),
    plain: fs.readFileSync(plainPath, "utf8").trimEnd(),
    typed: fs.readFileSync(typedPath, "utf8").trimEnd(),
  };
}

export function tabBlock(pair, { id = "", wrapMarkers = false } = {}) {
  const markerStart = wrapMarkers
    ? `<!-- NYRA_CODE_TABS ${pair.rel} -->\n`
    : "";
  const markerEnd = wrapMarkers ? `\n<!-- /NYRA_CODE_TABS -->` : "";
  const block = `<div class="code-tabs" data-code-tabs${id ? ` id="${escapeHtml(id)}"` : ""}>
  <div class="code-tabs-bar" role="tablist">
    <button type="button" class="code-tab active" role="tab" data-tab="easy" aria-selected="true">Without types</button>
    <button type="button" class="code-tab" role="tab" data-tab="typed" aria-selected="false">With types</button>
  </div>
  <div class="code-panel active" data-panel="easy" role="tabpanel"><pre><code>${escapeHtml(pair.plain)}</code></pre></div>
  <div class="code-panel" data-panel="typed" role="tabpanel" hidden><pre><code>${escapeHtml(pair.typed)}</code></pre></div>
</div>`;
  return `${markerStart}${block}${markerEnd}`;
}

export function stripInjectedTabs(html) {
  return html.replace(
    /<!-- NYRA_CODE_TABS examples\/[^\s]+ -->[\s\S]*?<!-- \/NYRA_CODE_TABS -->\n?/g,
    "",
  );
}
