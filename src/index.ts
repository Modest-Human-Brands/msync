import path from 'node:path'
import fs from 'node:fs/promises'
import chalk from 'chalk'
import { consola } from 'consola'
import addWatermarkImage from './utils/add-watermark-image'
import addWatermarkVideo from './utils/add-watermark-video'

interface WatermarkOptions {
  srcDir: string
  destDir: string
  watermarkPath: string
  position?: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right' | 'center'
  size?: number // percentage of image width
  opacity?: number // 0–1
}

const isImage = (file: string) => /\.(jpe?g|png)$/i.test(file)
const isVideo = (file: string) => /\.(mp4|mov|webm|mkv)$/i.test(file)

export async function getAllMedia(dir: string): Promise<string[]> {
  const entries = await fs.readdir(dir, { withFileTypes: true })
  const medias: string[] = []

  for (const e of entries) {
    const full = path.join(dir, e.name)
    if (e.isDirectory()) {
      medias.push(...(await getAllMedia(full)))
    } else if (isImage(e.name) || isVideo(e.name)) {
      medias.push(full)
    }
  }

  return medias
}

export async function addWatermarks(options: WatermarkOptions, onProgress?: (processed: number, total: number, file: string) => void) {
  const { srcDir, destDir, watermarkPath, position = 'bottom-right', size = 10, opacity = 0.1 } = options

  const allMedia = await getAllMedia(srcDir)
  const total = allMedia.length
  let processed = 0

  for (const srcPath of allMedia) {
    const relPath = path.relative(srcDir, srcPath)
    const destPath = path.join(destDir, relPath)

    await fs.mkdir(path.dirname(destPath), { recursive: true })

    const fileName = path.basename(srcPath)
    try {
      if (isImage(srcPath)) {
        // image-only pipeline
        await addWatermarkImage(srcPath, destPath, watermarkPath, position, size, opacity)
      } else if (isVideo(srcPath)) {
        // video-only pipeline
        await addWatermarkVideo(srcPath, destPath, watermarkPath, position, size, opacity)
      }

      processed++
      onProgress?.(processed, total, fileName) // ✅ tick progress here
    } catch (error) {
      consola.error(chalk.red(`❌ Failed: ${fileName}`), error)
    }
  }
}

function parseMarkdownTableRows(text: string) {
  const lines = text.split(/\r?\n/)
  const dataLines = lines.filter(
    (line) =>
      line.trim().startsWith('|') &&
      !line
        .split('|')
        .map((c) => c.trim())
        .every((c) => c === '' || /^-+$/.test(c.replace(/\s/g, '')))
  )
  return dataLines
    .slice(1)
    .map((line) =>
      line
        .split('|')
        .slice(1, -1)
        .map((s) => s.trim())
    )
    .filter((parts) => !parts.every((p) => p === ''))
}

function parseRangeList(text: string): { start: number; end: number }[] {
  if (!text) return []
  return text
    .split(/[;,]/)
    .map((p) => p.replace(/\(.*?\)/g, '').trim())
    .filter(Boolean)
    .map((p) => {
      const m = p.match(/(\d+)\s*[-–]\s*(\d+)/) || p.match(/(\d{3,})/)
      if (!m) return
      if (m.length >= 3) {
        const [a, b] = [Number.parseInt(m[1], 10), Number.parseInt(m[2], 10)]
        return { start: Math.min(a, b), end: Math.max(a, b) }
      }
      const single = Number.parseInt(m[1], 10)
      return { start: single, end: single }
    })
    .filter((item) => !!item)
}

async function listFilesRecursive(dir: string) {
  const out: string[] = []
  async function walk(p: string) {
    const entries = await fs.readdir(p, { withFileTypes: true })
    for (const e of entries) {
      const full = path.join(p, e.name)
      if (e.isDirectory()) await walk(full)
      else out.push(full)
    }
  }
  await walk(dir)
  return out
}

function filenameMatchesRanges(filename: string, ranges: { start: number; end: number }[]) {
  const base = filename.replace(/\.[^.]+$/, '') // strip extension
  const nums = Array.from(base.matchAll(/\d+/g), (m) => Number.parseInt(m[0], 10))
  return nums.some((n) => ranges.some((r) => n >= r.start && n <= r.end))
}

function sanitizePathParts(text: string) {
  return text
    .split(/[/\\]+/)
    .map((p) => p.trim())
    .filter(Boolean)
    .map((p) =>
      p
        .replace(/[<>:"|?*]/g, '')
        .replace(/\s+/g, ' ')
        .trim()
    )
}

export async function reorganize(srcDir: string, destDir: string, specFilePath: string) {
  const raw = await fs.readFile(specFilePath, 'utf8')
  const groups = parseMarkdownTableRows(raw).map((row) => ({
    shoot: row[0]?.trim(),
    productTag: row[1]?.trim() || 'unknown',
    productVariant: row[2]?.trim() || 'default',
    photoRanges: parseRangeList(row[3]?.trim() || ''),
    model: row[5]?.trim() || '',
  }))

  const allFiles = await listFilesRecursive(srcDir)

  for (const group of groups) {
    if (group.photoRanges.length === 0) {
      continue
    }

    const baseDest = path.join(destDir, ...sanitizePathParts(group.productTag), ...sanitizePathParts(group.productVariant))

    await fs.mkdir(baseDest, { recursive: true })

    for (const file of allFiles) {
      const fileName = path.basename(file)

      const isFileGroupMatch = filenameMatchesRanges(fileName, group.photoRanges)

      if (isFileGroupMatch) {
        const destPath = path.join(baseDest, fileName)

        await fs.mkdir(path.dirname(destPath), { recursive: true })
        await fs.copyFile(file, destPath)
      }
    }
  }
}
