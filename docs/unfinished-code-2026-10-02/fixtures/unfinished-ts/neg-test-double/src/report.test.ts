import { emptyRows } from "./__tests__/fakes";
import { toCsv } from "./report";

export const mockCsv = toCsv(emptyRows());
