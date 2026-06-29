import { defineCollection } from "astro:content";
import { docsLoader } from "@astrojs/starlight/loaders";
import { docsSchema } from "@astrojs/starlight/schema";

// Starlight (Astro 7) requires the `docs` content collection to be declared with
// its loader + schema; without this the collection is empty and no pages build.
export const collections = {
  docs: defineCollection({ loader: docsLoader(), schema: docsSchema() }),
};
