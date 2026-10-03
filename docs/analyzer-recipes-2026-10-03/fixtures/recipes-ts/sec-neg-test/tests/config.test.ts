declare function test(name: string, body: () => void): void;

const fixture = { user: "test", password: "test-password" };

test("fixture", () => {
  void fixture;
});
