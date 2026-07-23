export function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

const SEVERITY_ICON = { error: "⛔", warning: "⚠️", info: "ℹ️" };
const VERDICT_ICON = {
  conforms: "✅",
  violates: "❌",
  "not-targeted": "⊘",
  vacuous: "~",
};
const VERDICT_LABEL = {
  conforms: "Conforms",
  violates: "Violates",
  "not-targeted": "Not targeted",
  vacuous: "Vacuously conforms",
};

function renderSnippet(snippet) {
  const origin = snippet.origin === "data" ? "Data graph" : "Shapes graph";
  const highlightIndex = snippet.highlight ? snippet.turtle.indexOf(snippet.highlight) : -1;

  let turtleHtml;
  if (highlightIndex >= 0) {
    const before = snippet.turtle.slice(0, highlightIndex);
    const match = snippet.turtle.slice(highlightIndex, highlightIndex + snippet.highlight.length);
    const after = snippet.turtle.slice(highlightIndex + snippet.highlight.length);
    turtleHtml = `${escapeHtml(before)}<mark>${escapeHtml(match)}</mark>${escapeHtml(after)}`;
  } else {
    turtleHtml = escapeHtml(snippet.turtle);
  }

  return `
    <div class="snippet">
      <div class="snippet-origin">${escapeHtml(origin)}</div>
      <pre class="snippet-turtle">${turtleHtml}</pre>
      <div class="snippet-annotation">${escapeHtml(snippet.annotation)}</div>
    </div>
  `;
}

function renderDiagnosticBody(diag) {
  const parts = [];
  for (const snippet of diag.snippets ?? []) {
    parts.push(renderSnippet(snippet));
  }
  if (diag.expected != null) {
    parts.push(`<div class="diag-field"><strong>Expected:</strong> ${escapeHtml(diag.expected)}</div>`);
  }
  if (diag.actual != null) {
    parts.push(`<div class="diag-field"><strong>Actual:</strong> ${escapeHtml(diag.actual)}</div>`);
  }
  for (const note of diag.notes ?? []) {
    parts.push(`<div class="diag-note">${escapeHtml(note)}</div>`);
  }
  if (diag.help) {
    parts.push(`<div class="diag-help"><strong>Help:</strong> ${escapeHtml(diag.help)}</div>`);
  }
  return parts.join("");
}

function renderDiagnosticCard(diag, options = {}) {
  const icon = options.leadingIcon ?? SEVERITY_ICON[diag.severity] ?? "";
  const expanded = options.defaultExpanded ?? diag.severity === "error";
  const footer =
    diag.focus_node != null
      ? `<div class="diag-footer">
          <button type="button" class="focus-chip" data-focus="${escapeHtml(diag.focus_node)}" data-shape="${escapeHtml(diag.source_shape ?? "")}">
            Explain why &rarr; <code>${escapeHtml(diag.focus_node)}</code>
          </button>
        </div>`
      : "";

  return `
    <article class="diag-card" data-code="${escapeHtml(diag.code)}">
      <header class="diag-header">
        <span class="diag-icon">${icon}</span>
        <button type="button" class="code-badge" data-code="${escapeHtml(diag.code)}">${escapeHtml(diag.code)}</button>
        <span class="diag-title">${escapeHtml(diag.title)}</span>
      </header>
      <div class="diag-body${expanded ? "" : " hidden"}">
        ${renderDiagnosticBody(diag)}
      </div>
      ${footer}
    </article>
  `;
}

function renderWhyTraceCard(diag) {
  const verdict = diag.verdict ?? "not-targeted";
  const icon = `${VERDICT_ICON[verdict] ?? ""} ${VERDICT_LABEL[verdict] ?? verdict}`;
  return renderDiagnosticCard(diag, { leadingIcon: icon, defaultExpanded: true });
}

export function renderSummaryBanner(diags) {
  const conforms = !diags.some((d) => d.code.startsWith("V") && d.severity === "error");
  const errorCount = diags.filter((d) => d.severity === "error").length;
  const warningCount = diags.filter((d) => d.severity === "warning").length;
  const cls = conforms ? "banner ok" : "banner err";
  const headline = conforms ? "✓ Conforms" : "✗ Data does not conform";

  return `<div class="${cls}">
    <strong>${escapeHtml(headline)}</strong>
    <span class="banner-counts">${errorCount} error${errorCount === 1 ? "" : "s"}, ${warningCount} warning${warningCount === 1 ? "" : "s"}</span>
  </div>`;
}

export function renderDiagnosticsList(diags) {
  if (diags.length === 0) {
    return '<p class="empty">No diagnostics.</p>';
  }
  return diags.map((d) => renderDiagnosticCard(d)).join("");
}

function violationKey(node, shape) {
  return `${node} ${shape ?? ""}`;
}

export function renderShapesPanel(shapeTargets, diags) {
  if (shapeTargets.length === 0) {
    return '<p class="empty">No shapes with targets.</p>';
  }

  const violatingPairs = new Set(
    diags.filter((d) => d.focus_node != null).map((d) => violationKey(d.focus_node, d.source_shape))
  );

  return shapeTargets
    .map((entry) => {
      const chips = entry.targets
        .map((t) => {
          const flagged = violatingPairs.has(violationKey(t.node, entry.shape)) ? " flagged" : "";
          return `<button type="button" class="node-chip${flagged}" data-node="${escapeHtml(t.node)}" data-shape="${escapeHtml(entry.shape)}" data-kind="${escapeHtml(t.term_kind)}">${escapeHtml(t.node)}</button>`;
        })
        .join("");
      return `<div class="shape-row">
        <div class="shape-name">${escapeHtml(entry.shape)}</div>
        <div class="node-chips">${chips}</div>
      </div>`;
    })
    .join("");
}

export function renderExplainPanel(entry) {
  const component = entry.component
    ? `<p class="explain-component"><strong>Component:</strong> ${escapeHtml(entry.component)}</p>`
    : "";

  return `
    <h3>${escapeHtml(entry.code)}: ${escapeHtml(entry.title)}</h3>
    ${component}
    <p><a href="${escapeHtml(entry.spec_ref)}" target="_blank" rel="noopener">SHACL spec reference &rarr;</a></p>
    <p class="explain-explanation">${escapeHtml(entry.explanation)}</p>
    <div class="diag-field"><strong>Failing example</strong><pre class="snippet-turtle">${escapeHtml(entry.failing_example)}</pre></div>
    <div class="diag-field"><strong>Fixed example</strong><pre class="snippet-turtle">${escapeHtml(entry.fixed_example)}</pre></div>
  `;
}

export function renderWhyPanel(traceDiags, focusNode) {
  if (traceDiags.length === 0) {
    return `<p class="empty">No trace results for <code>${escapeHtml(focusNode)}</code>.</p>`;
  }
  return traceDiags.map((d) => renderWhyTraceCard(d)).join("");
}
