import java.io.IOException;
import java.nio.file.*;

class X {
    static void clean() throws IOException {
        Files.deleteIfExists(Path.of("tmp"));
    }
}
