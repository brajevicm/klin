export class CalcError extends Error {
  at: number;

  constructor(message: string, at: number) {
    super(message);
    this.name = "CalcError";
    this.at = at;
  }
}
