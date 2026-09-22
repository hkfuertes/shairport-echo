use echo_controls::{Ring, input_devices};

fn main() {
    let mut failed = false;

    match Ring::open_default() {
        Ok(ring) => println!("LED ring frame endpoint: {}", ring.path().display()),
        Err(error) => {
            eprintln!("LED ring preflight failed: {error}");
            failed = true;
        }
    }

    match input_devices() {
        Ok(devices) if devices.is_empty() => {
            eprintln!("Button preflight failed: no /dev/input/event* nodes");
            failed = true;
        }
        Ok(devices) => {
            println!("Input nodes (select the one reporting button key codes):");
            for device in devices {
                println!("  {}: {}", device.path.display(), device.name);
            }
        }
        Err(error) => {
            eprintln!("Button preflight failed: {error}");
            failed = true;
        }
    }

    if failed {
        std::process::exit(2);
    }
}
