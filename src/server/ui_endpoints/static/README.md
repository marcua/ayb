# Vendored UI assets

Third-party CSS, JavaScript, and fonts that the web UI loads (see
`templates/base.html`). They're embedded in the binary by
`static_files.rs` and served from `/static/...`, so pages don't depend
on a CDN at runtime.

Each directory is named for the package and version it came from, and
holds that package's license. Files are unmodified copies of the
package's published files, except `geist.css` as noted:

| File | Source |
|---|---|
| `franken-ui-2.0.0/core.min.css` | npm `franken-ui@2.0.0/dist/css/core.min.css` |
| `franken-ui-2.0.0/utilities.min.css` | npm `franken-ui@2.0.0/dist/css/utilities.min.css` |
| `franken-ui-2.0.0-internal.42/core.iife.js` | npm `franken-ui@2.0.0-internal.42/dist/js/core.iife.js` |
| `franken-ui-2.0.0-internal.42/icon.iife.js` | npm `franken-ui@2.0.0-internal.42/dist/js/icon.iife.js` |
| `geist-v5/geist-*.woff2` | The Geist font's per-script subsets, as Google Fonts (revision `v5`) serves them from `https://fonts.googleapis.com/css2?family=Geist:wght@100..900&display=swap` |
| `geist-v5/geist.css` | That URL's `@font-face` rules, with each `src` pointed at the local subset |
| `htmx-2.0.4/htmx.min.js` | npm `htmx.org@2.0.4/dist/htmx.min.js` |
| `htmx-ext-response-targets-2.0.3/response-targets.min.js` | npm `htmx-ext-response-targets@2.0.3/dist/response-targets.min.js` |

Geist is under the SIL Open Font License (`geist-v5/OFL.txt`). To update
it, fetch that Google Fonts URL with a current browser's User-Agent (it
serves `woff2` to modern browsers), download each subset it links to, and
point the copied rules at them.

To upgrade an npm package, download the new version's files (for example,
from `https://cdn.jsdelivr.net/npm/<package>@<version>/<path>`) into a
directory named for the new version, along with its license, and update
`static_files.rs`, `templates/base.html`, and this table. A new
directory name gives the files a new URL, which matters because they
are served with a one-year `immutable` cache lifetime.
