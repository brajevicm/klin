const RADIUS = 6371000;
const TURN = 360;

export interface Point {
  lat: number;
  lon: number;
}

/** The distance between two points, in whole metres. */
export function distance(a: Point, b: Point): number {
  const middle = ((a.lat + b.lat) * Math.PI) / (TURN * 2);
  const east = (((b.lon - a.lon) * Math.PI) / 180) * Math.cos(middle);
  const north = ((b.lat - a.lat) * Math.PI) / 180;
  return Math.round(Math.hypot(east, north) * RADIUS);
}

/** The compass direction from the first point to the second, in whole degrees. */
export function direction(a: Point, b: Point): number {
  const degrees = (Math.atan2(b.lon - a.lon, b.lat - a.lat) * 180) / Math.PI;
  return Math.round((degrees + TURN) % TURN);
}
