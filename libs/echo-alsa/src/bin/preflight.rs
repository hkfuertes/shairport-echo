use echo_alsa::preflight;

fn main() {
    match preflight() {
        Ok(report) => {
            println!("PCM configured without writing audio:");
            println!(
                "  {} Hz, {} channels, {} frames × {} periods",
                report.pcm.sample_rate,
                report.pcm.channels,
                report.pcm.period_frames,
                report.pcm.periods
            );
            println!("Mixer controls: {}", report.controls.len());
            for control in &report.controls {
                print!(
                    "  #{} {} ({:?}, count {}, access 0x{:x})",
                    control.numid,
                    control.name,
                    control.control_type,
                    control.count,
                    control.access
                );
                if !control.enum_items.is_empty() {
                    print!(" => {}", control.enum_items.join(", "));
                }
                println!();
            }
            if report.missing_controls.is_empty() {
                println!("All Echo profile candidate controls are present.");
            } else {
                eprintln!(
                    "Missing Echo profile candidate controls: {}",
                    report.missing_controls.join(", ")
                );
                std::process::exit(2);
            }
        }
        Err(error) => {
            eprintln!("Echo ALSA preflight failed: {error}");
            std::process::exit(1);
        }
    }
}
