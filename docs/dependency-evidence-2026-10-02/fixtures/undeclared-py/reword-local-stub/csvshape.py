class Schema:
    @classmethod
    def infer(cls, path):
        return cls()

    def validate(self, path):
        return True
