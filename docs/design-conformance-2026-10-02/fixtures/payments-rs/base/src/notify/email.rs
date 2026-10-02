use super::notifier::LegacyNotifier;

pub struct EmailNotifier;

impl LegacyNotifier for EmailNotifier {
    fn send(&self, to: &str, text: &str) {
        println!("mail to {to}: {text}");
    }
}
