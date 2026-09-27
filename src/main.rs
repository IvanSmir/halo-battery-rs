mod device;
mod providers;

use device::Provider;

fn main() {
    unsafe {
        let _ = windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED);
    }
    let mut providers: Vec<Box<dyn Provider>> = vec![
        Box::new(providers::logitech::LogitechProvider::new()),
        Box::new(providers::gamepad::GamepadProvider::new()),
    ];
    // Windows.Gaming.Input fills its controller list asynchronously
    std::thread::sleep(std::time::Duration::from_millis(1500));
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
