import { escapeHtml, tabBlock } from "./code-tabs.mjs";
import { methodLabel, methodSlug } from "./builtin-meta.mjs";

export function methodsGalleryBlock(pair, output) {
  const label = methodLabel(pair.title);
  const slug = methodSlug(pair.title);
  const outHtml = output
    ? `<p class="example-output-label">Output</p>
<pre class="example-output"><code>${escapeHtml(output)}</code></pre>`
    : "";
  return `
<h4 class="builtin-ex-title" id="ex-${slug}"><code>${escapeHtml(label)}</code></h4>
${tabBlock(pair)}
${outHtml}`;
}

export function stdlibGalleryBlock(pair) {
  return `
<h4 class="builtin-ex-title" id="ex-${pair.id}"><code>${escapeHtml(pair.title)}</code></h4>
${tabBlock(pair)}`;
}
