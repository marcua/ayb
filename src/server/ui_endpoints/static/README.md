# Vendored UI assets

Third-party CSS and JavaScript that the web UI loads (see
`templates/base.html`). They're embedded in the binary by
`static_files.rs` and served from `/static/...`, so pages don't depend
on a CDN at runtime.

Each directory is named for the npm package and version it came from,
and holds that package's license. Files are unmodified copies of the
package's published files:

| File | npm source |
|---|---|
| `franken-ui-2.0.0/core.min.css` | `franken-ui@2.0.0/dist/css/core.min.css` |
| `franken-ui-2.0.0/utilities.min.css` | `franken-ui@2.0.0/dist/css/utilities.min.css` |
| `franken-ui-2.0.0-internal.42/core.iife.js` | `franken-ui@2.0.0-internal.42/dist/js/core.iife.js` |
| `franken-ui-2.0.0-internal.42/icon.iife.js` | `franken-ui@2.0.0-internal.42/dist/js/icon.iife.js` |
| `htmx-2.0.4/htmx.min.js` | `htmx.org@2.0.4/dist/htmx.min.js` |
| `htmx-ext-response-targets-2.0.3/response-targets.min.js` | `htmx-ext-response-targets@2.0.3/dist/response-targets.min.js` |

To upgrade a package, download the new version's files (for example,
from `https://cdn.jsdelivr.net/npm/<package>@<version>/<path>`) into a
directory named for the new version, along with its license, and update
`static_files.rs`, `templates/base.html`, and this table. A new
directory name gives the files a new URL, which matters because they
are served with a one-year `immutable` cache lifetime.
