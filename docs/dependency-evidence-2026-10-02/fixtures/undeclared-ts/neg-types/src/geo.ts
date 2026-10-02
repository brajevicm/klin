import type { Feature } from "geojson";

export function name(feature: Feature): string {
  return String(feature.properties?.name);
}
