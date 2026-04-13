export type MediaType = 'photo' | 'video'

export type MediaMetadata = {
  size: number
  resolution: string
  aspectRatio: string
  bitDepth: string
  fps?: number
}

export type MediaItem = {
  slug: string
  type: MediaType
  title: string
  thumbnailUrl?: string
  metadata: MediaMetadata
}