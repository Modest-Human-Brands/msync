import { text, select } from '@clack/prompts'
import { defineCommand, runMain as _runMain } from 'citty'
import ora from 'ora'
import cliProgress from 'cli-progress'

import { name, description, version } from '../package.json'
import { addWatermarks, getAllImages } from '.'

export const main = defineCommand({
  meta: {
    name,
    description,
    version,
  },
  args: {
    src: { type: 'string', description: 'Source images directory' },
    dest: { type: 'string', description: 'Destination directory' },
    wm: { type: 'string', description: 'Watermark image path' },
    pos: { type: 'string', description: 'Watermark position' },
    size: { type: 'string', description: 'Watermark size (% of width)' },
    opacity: { type: 'string', description: 'Watermark opacity (0-1)' },
  },
  async run({ args }) {
    // If not provided via CLI, ask interactively
    const src = args.src ?? (await text({ message: 'Enter source images directory' }))
    const dest = args.dest ?? (await text({ message: 'Enter destination directory' }))
    const wm = args.wm ?? (await text({ message: 'Enter watermark image path' }))

    const pos =
      args.pos ??
      (await select({
        message: 'Select watermark position',
        initialValue: 'bottom-right',
        options: [
          { value: 'top-left', label: 'Top Left' },
          { value: 'top-right', label: 'Top Right' },
          { value: 'bottom-left', label: 'Bottom Left' },
          { value: 'bottom-right', label: 'Bottom Right' },
          { value: 'center', label: 'Center' },
        ],
      }))

    const size =
      Number.parseInt(args.size, 10) > 0 ? Number.parseInt(args.size, 10) : Number.parseInt((await text({ message: 'Enter watermark size (% of width)', initialValue: '20' })).toString(), 10)

    const opacity = args.opacity ? Number.parseFloat(args.opacity) : Number.parseFloat((await text({ message: 'Enter watermark opacity (0–1)', initialValue: '0.5' })).toString())

    /*  */
    const startTime = Date.now()

    const totalImages = (await getAllImages(src)).length

    const spinner = ora(`Processing ${totalImages} images...`).start()
    const bar = new cliProgress.SingleBar({ format: 'Progress |{bar}| {value}/{total} Images | Elapsed: {elapsed}s | ETA: {eta}s' }, cliProgress.Presets.legacy)
    bar.start(totalImages, 0)
    /*  */

    await addWatermarks(
      {
        srcDir: src,
        destDir: dest,
        watermarkPath: wm,
        position: pos as any,
        size,
        opacity,
      },
      (processed) => {
        bar.update(processed, { elapsed: ((Date.now() - startTime) / 1000).toFixed(1) })
        spinner.text = `Processing ${processed}/${totalImages} images...`
      }
    )

    bar.stop()
    spinner.succeed(`Done! Processed ${totalImages} images in ${((Date.now() - startTime) / 1000).toFixed(3)}s`)
  },
})

export const runMain = () => _runMain(main)
