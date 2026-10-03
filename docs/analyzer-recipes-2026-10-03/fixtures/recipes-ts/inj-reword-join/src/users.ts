import type { Db, Row } from "./db";

export interface User {
  id: number;
  name: string;
  email: string;
}

function toUser(row: Row): User {
  return { id: Number(row.id), name: String(row.name), email: String(row.email) };
}

export async function findByName(db: Db, name: string): Promise<User[]> {
  const sql = ["SELECT id, name, email FROM users WHERE name = '", name, "'"].join("");
  const rows = await db.query(sql);
  return rows.map(toUser);
}

export async function findById(db: Db, id: number): Promise<User | undefined> {
  const rows = await db.query("SELECT id, name, email FROM users WHERE id = $1", [id]);
  const first = rows[0];
  return first ? toUser(first) : undefined;
}
