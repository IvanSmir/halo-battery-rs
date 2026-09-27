mod device;
mod providers;

use device::Provider;

fn main() {
    let mut providers: Vec<Box<dyn Provider>> = vec![Box::new(providers::logitech::LogitechProvider::new())];
    for p in &mut providers {
        let devices = p.poll();
        for line in p.diagnostics() {
            println!("{line}");
        }
        for d in devices {
            println!("{d:?}");
        }
    }
}
