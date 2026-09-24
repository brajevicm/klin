const CHANNEL = 255;

export interface Rgb {
  r: number;
  g: number;
  b: number;
}

/** A colour with an opacity from 0 for clear to 1 for solid. */
export interface Rgba extends Rgb {
  alpha: number;
}

function hexOf(channel: number): string {
  return channel.toString(16).padStart(2, "0");
}

/** The colour as a lower-case `#rrggbb` code. */
export function toHex(color: Rgb): string {
  return "#" + hexOf(color.r) + hexOf(color.g) + hexOf(color.b);
}

/** The colour `weight` of the way from `a` to `b`, each channel rounded. */
export function mix(a: Rgb, b: Rgb, weight: number): Rgb {
  const towards = (from: number, to: number) => Math.round(from + (to - from) * weight);
  return { r: towards(a.r, b.r), g: towards(a.g, b.g), b: towards(a.b, b.b) };
}

function linear(channel: number): number {
  const share = channel / CHANNEL;
  return share <= 0.04045 ? share / 12.92 : ((share + 0.055) / 1.055) ** 2.4;
}

/** The relative luminance from 0 for black to 1 for white, to four places. */
export function luminance(color: Rgb): number {
  const value = 0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b);
  return Math.round(value * 10000) / 10000;
}

/** The solid colour a viewer sees when `top` is painted over the solid `bottom`. */
export function over(top: Rgba, bottom: Rgb): Rgb {
  const blend = (up: number, down: number) => Math.round(up * top.alpha + down * (1 - top.alpha));
  return { r: blend(top.r, bottom.r), g: blend(top.g, bottom.g), b: blend(top.b, bottom.b) };
}
