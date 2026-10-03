import DOMPurify from "dompurify";
import { marked, Renderer } from "marked";

const allowedTags = [
  "a", "blockquote", "br", "code", "dd", "del", "details", "div", "dl", "dt",
  "em", "h1", "h2", "h3", "h4", "h5", "h6", "hr", "i", "li", "ol", "p",
  "pre", "section", "span", "strong", "summary", "table", "tbody", "td", "th",
  "thead", "tr", "ul",
];

export function renderMarkdownPreview(
  source: string,
  imagePreviewDisabledLabel: string,
  checkedLabel: string,
  uncheckedLabel: string,
): string {
  const renderer = new Renderer();
  renderer.image = ({ text }) =>
    `<span class="markdown-image-placeholder">${escapeHtml(imagePreviewDisabledLabel)}${text ? `: ${escapeHtml(text)}` : ""}</span>`;
  renderer.checkbox = ({ checked }) =>
    `<span class="markdown-task-checkbox" aria-label="${escapeHtml(checked ? checkedLabel : uncheckedLabel)}">${checked ? "✓" : "□"}</span>`;

  const html = marked.parse(source, { async: false, gfm: true, breaks: false, renderer });
  return DOMPurify.sanitize(html, {
    ALLOWED_TAGS: allowedTags,
    ALLOWED_ATTR: ["align", "aria-label", "class", "href", "start", "title"],
    ALLOW_DATA_ATTR: false,
  });
}

function escapeHtml(value: string): string {
  return value.replace(/[&<>"']/g, character => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
  })[character]!);
}
