import org.junit.jupiter.api.*;

class XTest {
    @Disabled
    @Test
    void name() {
        Assertions.assertEquals("app", App.name());
    }
}
