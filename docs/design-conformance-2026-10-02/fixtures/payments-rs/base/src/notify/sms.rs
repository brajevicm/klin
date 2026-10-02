use super::notifier::LegacyNotifier;

pub struct SmsNotifier;

impl LegacyNotifier for SmsNotifier {
    fn send(&self, to: &str, text: &str) {
        println!("sms to {to}: {text}");
    }
}
