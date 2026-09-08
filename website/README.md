# OmaSend website

Static Astro site, following the Astro + Bun + GitHub Pages setup in `../gpui-omarchy/website`. Fonts are bundled locally. English lives at `/omasend/`, Simplified Chinese at `/omasend/zh-CN/`.

```sh
cd website
bun install --frozen-lockfile
bun run dev
bun run build
bun x playwright install chromium
bun run test
```

The default base is `/omasend`. Set `BASE_PATH` and `SITE_URL` together for another host. The production browser tests deliberately verify the GitHub Pages path.

`.github/workflows/website.yml` builds and tests pull requests, then deploys `main` using GitHub Pages. Repository Settings → Pages → Source must be **GitHub Actions**. No custom domain or external hosting credentials are required. The workflow does not publish a preview from a pull request.

Installer commands use `install.sh` and `install.ps1` from `main`; these become usable after the implementation is merged and a release is published. macOS downloads are `.tar.gz`, Linux `.tar.gz`, and Windows `.zip`.

The network artwork is an illustration of local transfer, not a screenshot or a browser implementation of OmaSend. `public/icon.png` is copied unchanged from `assets/omasend.png`; its provenance is recorded in `assets/icon-provenance.txt`. The light/dark toggle follows the system until an explicit choice is saved, using Tokyo Night and Flexoki Light palettes. Language is selected from the header menu. The site does not claim mobile interoperability or Windows runtime testing has been completed.
