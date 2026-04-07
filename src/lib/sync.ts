import { readDir } from "@tauri-apps/plugin-fs";
import { join, resolve } from "@tauri-apps/api/path";
import { convertFileSrc } from "@tauri-apps/api/core";

export type MediaFile = {
  path: string;
  name: string;
  type: string;
  size: number;
  url: string;
};

export async function listFilesRecursively(dir: string): Promise<MediaFile[]> {
  const entries = await readDir(dir);
  const result: MediaFile[] = [];

  for (const entry of entries) {
    const fullPath = await join(dir, entry.name);
    if (entry.isDirectory) {
      result.push(...(await listFilesRecursively(fullPath)));
    } else {
      result.push({
        path: fullPath,
        name: entry.name,
        type: entry.name.split(".").pop()?.toLowerCase() || "",
        size: 0,
        url: convertFileSrc(await resolve(fullPath)),
      });
    }
  }
  return result;
}