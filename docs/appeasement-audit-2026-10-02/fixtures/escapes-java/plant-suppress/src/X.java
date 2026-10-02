import java.util.List;

class X {
    @SuppressWarnings("unchecked")
    static List<String> names(Object raw) {
        return (List<String>) raw;
    }
}
