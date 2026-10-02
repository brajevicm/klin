use super::paypal::PayPalProvider;
use super::provider::PaymentProvider;
use super::stripe::StripeProvider;

pub fn providers() -> Vec<(&'static str, Box<dyn PaymentProvider>)> {
    vec![("stripe", Box::new(StripeProvider)), ("paypal", Box::new(PayPalProvider))]
}
