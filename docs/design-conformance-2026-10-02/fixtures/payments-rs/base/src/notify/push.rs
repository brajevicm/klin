use super::notifier::Notifier;

pub struct PushNotifier;

impl Notifier for PushNotifier {
    fn deliver(&self, to: &str, text: &str) -> crate::Result<()> {
        println!("push to {to}: {text}");
        Ok(())
    }
}
