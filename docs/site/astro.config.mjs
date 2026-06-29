// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

// GitHub Pages base path: the site is served from
// https://<owner>.github.io/prompting-press/. `site` + `base` make internal
// links + assets resolve under that subpath. Adjust `site` if a custom domain
// is configured later.
export default defineConfig({
  site: "https://srobroek.github.io",
  base: "/prompting-press",
  integrations: [
    starlight({
      title: "Prompting Press",
      description:
        "A typed, versioned, variant-aware prompt-template library for Rust, Python, and TypeScript.",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/srobroek/prompting-press",
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
            { label: "Prompt definition (shape)", link: "/reference/prompt-definition/" },
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
