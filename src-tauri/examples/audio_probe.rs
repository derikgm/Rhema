// Utilidad de diagnóstico: lista dispositivos de entrada y formatos soportados.
use cpal::traits::{DeviceTrait, HostTrait};

fn main() {
    let host = cpal::default_host();
    println!("host: {:?}", host.id());
    match host.default_input_device() {
        Some(dev) => {
            println!("default input: {dev}");
            match dev.default_input_config() {
                Ok(c) => println!(
                    "  default config: {:?} rate={} ch={}",
                    c.sample_format(),
                    c.sample_rate(),
                    c.channels()
                ),
                Err(e) => println!("  default config error: {e}"),
            }
            match dev.supported_input_configs() {
                Ok(cfgs) => {
                    for c in cfgs {
                        println!(
                            "  cfg: {:?} ch={} min_rate={} max_rate={}",
                            c.sample_format(),
                            c.channels(),
                            c.min_sample_rate(),
                            c.max_sample_rate()
                        );
                    }
                }
                Err(e) => println!("  supported configs error: {e}"),
            }
        }
        None => println!("NO default input device"),
    }
}
