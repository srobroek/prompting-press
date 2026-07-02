// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import { unified } from "@astrojs/markdown-remark";
import { remarkPrefixInternalLinks } from "./scripts/lib/remark-prefix-internal-links.mjs";

// ---------------------------------------------------------------------------
// TC01: Per-version build parameterisation (Phase 7).
//
// The multi-build script (scripts/build-versions.mjs) sets these env vars to
// produce a self-contained Starlight site for each version prefix:
//
//   PP_DOCS_BASE     — Astro `base`, e.g. "/next/" or "/v0.1/". Defaults to
//                      "/" so that the normal dev / single-build experience is
//                      unchanged when neither var is set.
//   PP_DOCS_VERSION  — human-visible version label baked into the page title,
//                      e.g. "v0.1" or "next". Defaults to "" (bare "Prompting
//                      Press" title).
//
// Unset → current dev/single-build behaviour is preserved (base="/").
// ---------------------------------------------------------------------------
const docsBase    = process.env.PP_DOCS_BASE    ?? "/";
const docsVersion = process.env.PP_DOCS_VERSION ?? "";

// Normalize base: must start and end with "/" for Astro.
const normalizedBase = docsBase.startsWith("/") ? docsBase : `/${docsBase}`;

const siteTitle = docsVersion
  ? `Prompting Press ${docsVersion}`
  : "Prompting Press";

// GitHub Pages root site: the repo is named `prompting-press.github.io`, so it
// is served at the org root https://prompting-press.github.io/ (no subpath).
// `site` is the canonical origin; `base` is set per version for multi-builds.
// Adjust `site` if a custom domain is configured later.
export default defineConfig({
  site: "https://prompting-press.github.io",
  base: normalizedBase,
  markdown: {
    // Rewrites root-absolute internal links (e.g. /guides/variants/) in every
    // content page to carry this build's version prefix — see the plugin's
    // header comment for why Starlight's own base-prefixing doesn't do this.
    // `markdown.remarkPlugins` is deprecated in Astro 7; plugins now attach to
    // an explicit `unified()` processor instead (@astrojs/markdown-remark).
    processor: unified({
      remarkPlugins: [remarkPrefixInternalLinks(normalizedBase)],
    }),
  },
  integrations: [
    starlight({
      components: {
        // T009: inject VersionSelect dropdown into every page header
        SiteTitle: "./src/components/overrides/SiteTitle.astro",
        // T010: inject freshness footer ("docs current as of X.Y.Z") into every page
        Footer: "./src/components/overrides/Footer.astro",
        // TC06: noindex + self-canonical meta injected into <head> per build
        Head: "./src/components/overrides/Head.astro",
        // TC05: server-rendered old-version banner prepended to article body
        MarkdownContent: "./src/components/overrides/MarkdownContent.astro",
      },
      title: siteTitle,
      description:
        "A typed, versioned, variant-aware prompt-template library for Rust, Python, and TypeScript.",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/prompting-press/prompting-press",
        },
      ],
      sidebar: [
        { label: "Overview", link: "/" },
        {
          label: "Getting started",
          items: [
            { label: "Rust", link: "/getting-started/rust/" },
            { label: "Python", link: "/getting-started/python/" },
            { label: "TypeScript", link: "/getting-started/typescript/" },
          ],
        },
        {
          label: "Guides",
          items: [
            { label: "Variants", link: "/guides/variants/" },
            { label: "Deriving a prompt", link: "/guides/derive/" },
            { label: "Metadata", link: "/guides/metadata/" },
            { label: "Compose multi-message prompts", link: "/guides/composition/" },
            { label: "The advisory guard", link: "/guides/guard/" },
            { label: "Lint prompts in CI", link: "/guides/lint-in-ci/" },
          ],
        },
        { label: "Template features", link: "/templates/" },
        {
          label: "API reference",
          items: [
            { label: "Prompt Definition", link: "/reference/prompt-definition/" },
            { label: "Rust", link: "/reference/rust/" },
            { label: "Python", link: "/reference/python/" },
            { label: "TypeScript", link: "/reference/typescript/" },
          ],
        },
        { label: "FAQ", link: "/faq/" },
      ],
    }),
  ],
});
