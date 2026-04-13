export function isImageFile(path: string) {
  return /\.(png|jpe?g|webp|gif|bmp|svg|avif)$/i.test(path);
}

export function isVideoFile(path: string) {
  return /\.(mp4|mov|avi|mkv|webm)$/i.test(path);
}

export function isMediaFile(path: string) {
  return isImageFile(path) || isVideoFile(path);
}