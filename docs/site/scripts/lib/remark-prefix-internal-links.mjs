/**
 * remark-prefix-internal-links.mjs
 *
 * Content pages (src/content/docs/**, and the frozen src/versions/vX.Y/**
 * trees) link to each other with root-absolute paths, e.g.
 * `[Variants](/guides/variants/)`. Starlight prefixes its OWN generated chrome
 * (sidebar, ToC, pagination) with the build's `base` automatically, but plain
 * markdown links inside article bodies are passed through untouched — under
 * a per-version build (base="/v0.2/", "/next/", ...) those links resolve at
 * the site root instead of inside the version, which 404s on the static host.
 *
 * This remark plugin rewrites every root-absolute internal link (and image
 * src) in the markdown AST to be prefixed with the active build's base,
 * mirroring what Starlight already does for its own chrome. Runs identically
 * for every version build (next/v0.1/v0.2/...) via build-versions.mjs, so it
 * fixes already-frozen snapshots too without touching their content.
 */

import { visit } from "unist-util-visit";

const EXTERNAL_RE = /^([a-z][a-z0-9+.-]*:|\/\/)/i;

/**
 * @param {string} base Astro `base`, normalized to start+end with "/" (e.g. "/v0.2/", "/next/", "/").
 * @returns {() => (tree: import("mdast").Root) => void}
 */
export function remarkPrefixInternalLinks(base) {
  // Bare root base ("/") needs no rewriting — this is the unversioned
  // single-build dev/preview path.
  if (base === "/") {
    return () => () => {};
  }

  const prefix = base.slice(0, -1); // "/v0.2/" -> "/v0.2"

  function rewrite(url) {
    if (typeof url !== "string" || url === "") return url;
    if (!url.startsWith("/")) return url; // relative, hash-only, mailto:, etc.
    if (EXTERNAL_RE.test(url)) return url; // protocol-relative "//host/..."
    if (url === prefix || url.startsWith(`${prefix}/`)) return url; // already prefixed
    return `${prefix}${url}`;
  }

  // link/image carry `url` directly; linkReference/imageReference resolve
  // through a `definition` node instead — covering "link", "image", and
  // "definition" is exhaustive for markdown's URL-bearing node types.
  return function attacher() {
    return function transformer(tree) {
      visit(tree, (node) => {
        if (node.type === "link" || node.type === "image" || node.type === "definition") {
          node.url = rewrite(node.url);
        }
      });
    };
  };
}
