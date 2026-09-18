use notes::registry::dispatch;
use notes::store::Store;

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let mut store = Store::new();
    println!("{}", dispatch(&arguments, &mut store));
}
