import path from 'node:path'
import fs from 'node:fs/promises'
import { createCanvas, loadImage } from '@napi-rs/canvas'

export type Position = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right' | 'center'

export default async function (srcPath: string, destPath: string, watermarkPath: string, position: Position, size: number, opacity: number) {
  const image = await loadImage(srcPath)
  const watermark = await loadImage(watermarkPath)

  const canvas = createCanvas(image.width, image.height)
  const ctx = canvas.getContext('2d')

  ctx.drawImage(image, 0, 0, image.width, image.height)

  const wmarkWidth = Math.floor((image.width * size) / 100)
  const scale = wmarkWidth / watermark.width
  const wmarkHeight = Math.floor(watermark.height * scale)

  let dx = 0,
    dy = 0
  switch (position) {
    case 'top-left': {
      dx = 10
      dy = 10
      break
    }
    case 'top-right': {
      dx = image.width - wmarkWidth - 10
      dy = 10
      break
    }
    case 'bottom-left': {
      dx = 10
      dy = image.height - wmarkHeight - 10
      break
    }
    case 'center': {
      dx = (image.width - wmarkWidth) / 2
      dy = (image.height - wmarkHeight) / 2
      break
    }
    default: {
      dx = image.width - wmarkWidth - 10
      dy = image.height - wmarkHeight - 10
    }
  }

  ctx.globalAlpha = Math.max(0, Math.min(1, opacity))
  ctx.drawImage(watermark, 0, 0, watermark.width, watermark.height, dx, dy, wmarkWidth, wmarkHeight)
  ctx.globalAlpha = 1

  await fs.mkdir(path.dirname(destPath), { recursive: true })
  const buffer = /\.(jpe?g)$/i.test(destPath) ? canvas.toBuffer('image/jpeg') : canvas.toBuffer('image/png')

  await fs.writeFile(destPath, buffer)
}
