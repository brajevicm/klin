import java.nio.file.*;

class X {
    static void clean() {
        try {
            Files.delete(Path.of("tmp"));
        } catch (Exception ignored) {
        }
    }
}
