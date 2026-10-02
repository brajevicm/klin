use super::notifier::Notifier;

pub struct SlackNotifier;

impl Notifier for SlackNotifier {
    fn deliver(&self, to: &str, text: &str) -> crate::Result<()> {
        println!("slack to {to}: {text}");
        Ok(())
    }
}
