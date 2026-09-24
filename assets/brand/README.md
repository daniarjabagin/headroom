# Headroom brand assets

Mark H01 / Reserve and the `headroom by asteru studio` wordmark, from the Asteru Studio brandbook 1.0
(approved 2026-09-24). Copied unmodified except `headroom-symbolic.svg`, whose transforms are flattened
into one path.

| File | Use |
| --- | --- |
| `headroom-logo-on-dark.svg` | horizontal logo with credit, Paper plates and text, for dark backgrounds |
| `headroom-logo-on-light.svg` | horizontal logo with credit, Graphite plates and text, for light backgrounds |
| `headroom-symbol-on-dark.svg` | mark only, transparent, for dark backgrounds |
| `headroom-symbol-on-light.svg` | mark only, transparent, for light backgrounds |
| `headroom-symbolic.svg` | monochrome 16 px panel mark (`#bebebe`, recolored by the toolkit) |
| `headroom-app-dark.svg` | square app icon, Graphite plate (the default) |
| `headroom-app-light.svg` | square app icon, Paper plate |
| `headroom-social-1200x630.png` | GitHub social preview and link cards |

Colors: Graphite `#151617`, Paper `#F0F0ED`, Ice `#A9B5FF` (the upper right segment only).

README header, switching with the viewer's theme:

```html
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/headroom-logo-on-dark.svg">
  <img alt="headroom by asteru studio" src="assets/brand/headroom-logo-on-light.svg" width="318">
</picture>
```

Rules: the logo with credit from 280 px wide, the mark from 16 px (panels use only the symbolic mark).
Keep 1X clear space around the visible contour, X = plate width. Do not stretch, re-slant or recolor
the plates, close the gap, or add glow, gradient, stroke or shadow.

Platform copies: `shell/gnome/icons/` and `shell/plasma/package/contents/icons/`
(`headroom-symbolic.svg`), `packaging/icons/headroom.svg` (Linux hicolor app icon on a rounded tile),
`shell/macos/Icon/` (macOS app icon on the 824/1024 grid; rerender with `script/render-icon.sh`) and
`BrandMark` in the macOS HeadroomKit (the symbolic path as data; a test keeps every copy identical).

## License

© Asteru Studio / Daniar Jabagin, all rights reserved. The Headroom name, logo, mark and wordmark are
not covered by the MIT license that applies to the source code. You may use them unmodified to refer
to Headroom; any other use needs written permission.
