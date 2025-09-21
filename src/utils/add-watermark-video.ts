import { execa } from 'execa'

import { Position } from './add-watermark-image'

const positions: Record<Position, string> = {
  'top-left': '10:10',
  'top-right': 'main_w-w-10:10',
  'bottom-left': '10:main_h-h-10',
  'bottom-right': 'main_w-w-10:main_h-h-10',
  center: '(main_w-w)/2:(main_h-h)/2',
}

export default async function (srcPath: string, destPath: string, watermarkPath: string, position: Position, size: number, opacity: number, onProgress?: (frame: number) => void) {
  const probe = await execa('ffprobe', ['-v', 'error', '-select_streams', 'v:0', '-show_entries', 'stream=width,height', '-of', 'json', srcPath])
  const info = JSON.parse(probe.stdout)
  const videoWidth = info?.streams?.[0]?.width
  if (!videoWidth) throw new Error('ffprobe failed to get video width')
  const wmWidth = Math.max(1, Math.round(videoWidth * (size / 100)))

  const overlay = positions[position]

  const filter = `[1]scale=${wmWidth}:-1[wm];[wm]format=rgba,colorchannelmixer=aa=${opacity}[wm_op];[0][wm_op]overlay=${overlay}`

  const ffmpegProcess = execa('ffmpeg', ['-i', srcPath, '-i', watermarkPath, '-filter_complex', filter, '-c:a', 'copy', '-y', destPath])

  if (onProgress) {
    ffmpegProcess.stderr?.on('data', (data: Buffer) => {
      const str = data.toString()
      const match = str.match(/frame=\s*(\d+)/)
      if (match) {
        onProgress(Number(match[1]))
      }
    })
  }

  await ffmpegProcess
}
