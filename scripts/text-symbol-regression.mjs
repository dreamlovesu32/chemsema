import fs from "node:fs";

import {
  normalizeSharedGlyphProfiles,
} from "../viewer/text_metrics.js";

const profiles = normalizeSharedGlyphProfiles(JSON.parse(fs.readFileSync("shared/glyph_profiles.json", "utf8")));
const catalog = JSON.parse(fs.readFileSync("shared/text_symbols.json", "utf8"));

for (const group of catalog.groups || []) {
  for (const character of Array.from(group.characters || "")) {
    const profile = profiles.specials[character];
    if (!profile?.visible || !(profile.advanceEm > 0) || !(profile.inkBottomEm > profile.inkTopEm)) {
      throw new Error(`missing explicit profile for ${character} in ${group.id}: ${JSON.stringify(profile)}`);
    }
  }
}

console.log("text symbol regression checks passed");
