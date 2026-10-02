pub trait LegacyNotifier {
    fn send(&self, to: &str, text: &str);
}

pub trait Notifier {
    fn deliver(&self, to: &str, text: &str) -> crate::Result<()>;
}
