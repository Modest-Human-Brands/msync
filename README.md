<p align="center">
  <picture>
    <source srcset="./public/logo-dark.png" media="(prefers-color-scheme: dark)">
    <source srcset="./public/logo-light.png" media="(prefers-color-scheme: light)">
    <img src="./public/logo-light.png" alt="Logo" width="128">
  </picture>
</p>

# RCP Tools

[![npm version](https://img.shields.io/npm/v/@shba007/rcp-tools?color=blue)](https://npmjs.com/package/@shba007/rcp-tools)
[![npm downloads](https://img.shields.io/npm/dm/@shba007/rcp-tools?color=blue)](https://npmjs.com/package/@shba007/rcp-tools)
[![License](https://img.shields.io/npm/l/@shba007/rcp-tools?color=blue)](https://github.com/shba007/rcp-tools?tab=MIT-1-ov-file)

> Print your favorite framework info into cli

<video controls src="public/preview.mp4" title="Demo" loop muted autoplay></video>

## Usage (CLI)

Globally run print with `npx`:

```sh
npx @shba007/rcp-tools@latest
```

or

```sh
npx @shba007/rcp-tools@latest -f [framework]
```

    Options:
    -f, --framework prints the framework's logo and details

Use `npx @shba007/rcp-tools print --help` for more usage info.

## Usage (API)

Install package:

```sh
# ✨ Auto-detect
npx nypm install @shba007/rcp-tools

# npm
npm install @shba007/rcp-tools

# yarn
yarn add @shba007/rcp-tools

# pnpm
pnpm install @shba007/rcp-tools

# bun
bun install @shba007/rcp-tools
```

Import:

**ESM** (Node.js, Bun)

```js
import { tools } from '@shba007/rcp-tools'
```

**CommonJS** (Legacy Node.js)

```js
const { tools } = require('@shba007/rcp-tools')
```

**CDN** (Deno, Bun and Browsers)

```js
import { tools } from 'https://esm.sh/@shba007/rcp-tools'
```

## Development

<details>

<summary>local development</summary>

- Clone this repository
- Install latest LTS version of [Node.js](https://nodejs.org/en/)
- Enable [Corepack](https://github.com/nodejs/corepack) using `corepack enable`
- Install dependencies using `pnpm install`
- Run interactive tests using `pnpm dev`

</details>

## License

Published under the [MIT](https://github.com/shba007/rcp-tools/blob/main/LICENSE) license.
<br><br>
<a href="https://github.com/shba007/rcp-tools/graphs/contributors">
<img src="https://contrib.rocks/image?repo=shba007/rcp-tools" />
</a>
