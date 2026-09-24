export interface User {
  id: string;
  hash: string;
}

export type UserStore = Map<string, User>;

export function addUser(store: UserStore, id: string, hash: string): void {
  store.set(id, { id, hash });
}

export function findUser(store: UserStore, id: string): User | undefined {
  return store.get(id);
}
