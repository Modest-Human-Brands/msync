import sharp from 'sharp'
import path from 'node:path'
import fs from 'node:fs/promises'
import chalk from 'chalk'
import { consola } from 'consola'

interface WatermarkOptions {
  srcDir: string
  destDir: string
  watermarkPath: string
  position?: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right' | 'center'
  size?: number // percentage of image width
  opacity?: number // 0–1
}

// helper: count images recursively
export async function getAllImages(dir: string): Promise<string[]> {
  const entries = await fs.readdir(dir, { withFileTypes: true })
  const images: string[] = []

  for (const e of entries) {
    const full = path.join(dir, e.name)
    if (e.isDirectory()) {
      images.push(...(await getAllImages(full)))
    } else if (/\.(jpe?g|png)$/i.test(e.name)) {
      images.push(full)
    }
  }

  return images
}

export async function addWatermarks(options: WatermarkOptions, onProgress?: (processed: number, total: number, file: string) => void) {
  const { srcDir, destDir, watermarkPath, position = 'bottom-right', size = 10, opacity = 0.1 } = options

  const allImages = await getAllImages(srcDir)
  const total = allImages.length
  let processed = 0

  for (const srcPath of allImages) {
    const relPath = path.relative(srcDir, srcPath)
    const destPath = path.join(destDir, relPath)

    await fs.mkdir(path.dirname(destPath), { recursive: true })

    const fileName = path.basename(srcPath)

    try {
      const img = sharp(srcPath)
      const metadata = await img.metadata()
      if (!metadata.width) continue

      const watermark = await sharp(watermarkPath)
        .resize(Math.floor((metadata.width * size) / 100))
        .removeAlpha()
        .ensureAlpha(opacity)
        .toBuffer()

      let gravity: sharp.Gravity = 'southeast'
      switch (position) {
        case 'top-left': {
          gravity = 'northwest'
          break
        }
        case 'top-right': {
          gravity = 'northeast'
          break
        }
        case 'bottom-left': {
          gravity = 'southwest'
          break
        }
        case 'center': {
          gravity = 'center'
          break
        }
      }

      await img.composite([{ input: watermark, gravity, blend: 'over' }]).toFile(destPath)

      processed++
      onProgress?.(processed, total, fileName) // ✅ tick progress here
    } catch (error) {
      consola.error(chalk.red(`❌ Failed: ${fileName}`), error)
    }
  }
}
