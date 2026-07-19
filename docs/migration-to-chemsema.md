# Project rename: ChemCore to ChemSema

The project was renamed from **ChemCore** to **ChemSema** on July 19, 2026.
The new name better reflects the project's focus: chemical meaning, document
semantics, and reliable operations across people, software, and agents.

The rename is explicit and does not rewrite Git history. Current source code,
package and crate names, commands, environment variables, documentation, and
repository paths use `ChemSema` or `chemsema`. Existing commits and tags remain
unchanged so their hashes, signatures, and provenance stay intact.

Compatibility commitments:

- GitHub's old repository URL is retained as a rename redirect and the retired
  repository name will not be reused.
- The old GitHub Pages path is served by a permanent compatibility page that
  forwards visitors to <https://dreamlovesu32.github.io/chemsema/>.
- Both routes are checked before local commits and by a daily GitHub Actions
  monitor so a future platform-side change is detected quickly.
- Existing document extensions such as `.ccjs` and `.ccjz` remain unchanged.

The public ChemSema release line restarts at `1.0.0-beta.1`. Because the old
Git history already contains a `v1.0.0-beta.1` tag, the new brand uses the
unambiguous tag `chemsema-v1.0.0-beta.1`.
