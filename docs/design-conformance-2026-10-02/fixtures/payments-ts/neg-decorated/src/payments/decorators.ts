const registered = new Map<string, unknown>();

export function Provider(name: string) {
  return (target: unknown) => {
    registered.set(name, target);
  };
}
