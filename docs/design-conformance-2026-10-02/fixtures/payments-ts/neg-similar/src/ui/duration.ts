export function formatDuration(seconds: number, style: string): string {
  const late = seconds < 0;
  const total = Math.abs(seconds);
  const minutes = Math.floor(total / 60);
  const rest = total % 60;
  const hours = Math.floor(minutes / 60);
  const clock = `${hours}:${(minutes % 60).toString().padStart(2, "0")}`;
  const padded = rest < 10 ? `0${rest}` : `${rest}`;
  const unit = style === "short" ? "m" : style === "long" ? " min" : ` ${style.toUpperCase()}`;
  if (late) {
    return `late ${clock}${unit}${padded}s`;
  }
  return `${clock}${unit}${padded}s`;
}
