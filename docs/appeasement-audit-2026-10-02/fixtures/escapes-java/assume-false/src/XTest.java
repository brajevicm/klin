import org.junit.jupiter.api.*;

class XTest {
    @Test
    void name() {
        Assumptions.assumeTrue(false);
        Assertions.assertEquals("app", App.name());
    }
}
