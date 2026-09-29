//! The diagnostics report as text. Pure: [`super::collect`] fills it in.

use std::fmt::{self, Display, Formatter, Write};

use super::descriptor::{self, Summary};
use crate::platform::bluetooth::BluetoothDevice;

/// One HID interface as the system lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HidInterface {
    pub vendor_id: u16,
    pub product_id: u16,
    pub manufacturer: String,
    pub product: String,
    pub bus: String,
    pub interface: i32,
    pub usage_page: u16,
    pub usage: u16,
    /// The report descriptor, or why it could not be read.
    pub descriptor: Result<Vec<u8>, String>,
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub app_version: String,
    pub windows: String,
    pub date: String,
    /// What the providers saw, as they describe it (`--list`).
    pub providers: Vec<String>,
    pub hid: Vec<HidInterface>,
    pub bluetooth: Vec<BluetoothDevice>,
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::new();
    for (i, chunk) in bytes.chunks(16).enumerate() {
        if i > 0 {
            out.push_str("\n      ");
        }
        let line: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        out.push_str(&line.join(" "));
    }
    out
}

fn pages(summary: &Summary) -> String {
    summary
        .pages
        .iter()
        .map(|&p| match descriptor::page_name(p) {
            "" => format!("0x{p:04X}"),
            name => format!("0x{p:04X} {name}"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Bluetooth nodes worth showing: the device itself (…\DEV_…) and anything
/// with a battery, not every profile it offers.
fn is_device_node(d: &BluetoothDevice) -> bool {
    d.battery.is_some() || d.instance_id.to_ascii_uppercase().contains(r"\DEV_")
}

impl Report {
    /// HID interfaces grouped by device (vendor and product id), in order.
    fn devices(&self) -> Vec<((u16, u16), Vec<&HidInterface>)> {
        let mut groups: Vec<((u16, u16), Vec<&HidInterface>)> = Vec::new();
        for h in &self.hid {
            let key = (h.vendor_id, h.product_id);
            match groups.iter_mut().find(|(k, _)| *k == key) {
                Some((_, list)) => list.push(h),
                None => groups.push((key, vec![h])),
            }
        }
        groups
    }

    /// Devices whose descriptor declares a battery usage.
    pub fn battery_hints(&self) -> Vec<&HidInterface> {
        self.hid
            .iter()
            .filter(|h| h.descriptor.as_ref().is_ok_and(|d| descriptor::summarize(d).mentions_battery()))
            .collect()
    }
}

impl Display for Report {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        writeln!(s, "Halo Battery - informe de diagnóstico")?;
        writeln!(s, "Versión {} · {} · {}", self.app_version, self.windows, self.date)?;
        writeln!(s)?;
        writeln!(s, "Este informe solo lee información: no se envió nada a ningún dispositivo.")?;
        writeln!(s, "Incluye los nombres de tus dispositivos y sus identificadores (VID/PID),")?;
        writeln!(s, "pero no números de serie.")?;

        writeln!(s, "\n== Lo que Halo Battery reconoce ==")?;
        if self.providers.is_empty() {
            writeln!(s, "  (nada)")?;
        }
        for line in &self.providers {
            writeln!(s, "  {line}")?;
        }

        let hints = self.battery_hints();
        writeln!(s, "\n== Dispositivos HID que declaran batería ==")?;
        if hints.is_empty() {
            writeln!(s, "  Ninguno declara un uso de batería estándar en su descriptor.")?;
        }
        for h in hints {
            writeln!(
                s,
                "  {:04X}:{:04X} {} {} (interfaz {}, página 0x{:04X})",
                h.vendor_id, h.product_id, h.manufacturer, h.product, h.interface, h.usage_page
            )?;
        }

        writeln!(s, "\n== Dispositivos HID ==")?;
        for ((vid, pid), interfaces) in self.devices() {
            let first = interfaces[0];
            writeln!(s, "\n[{vid:04X}:{pid:04X}] {} {}", first.manufacturer, first.product)?;
            for h in interfaces {
                writeln!(
                    s,
                    "  - bus {} · interfaz {} · página 0x{:04X} · uso 0x{:04X}",
                    h.bus, h.interface, h.usage_page, h.usage
                )?;
                match &h.descriptor {
                    Ok(d) => {
                        let summary = descriptor::summarize(d);
                        writeln!(s, "    páginas: {}", pages(&summary))?;
                        if summary.mentions_battery() {
                            writeln!(s, "    >> declara batería")?;
                        }
                        writeln!(s, "    descriptor ({} bytes):\n      {}", d.len(), hex(d))?;
                    }
                    Err(e) => writeln!(s, "    descriptor: no disponible ({e})")?,
                }
            }
        }

        writeln!(s, "\n== Bluetooth ==")?;
        let bt: Vec<&BluetoothDevice> = self.bluetooth.iter().filter(|d| is_device_node(d)).collect();
        if bt.is_empty() {
            writeln!(s, "  Ningún dispositivo Bluetooth emparejado.")?;
        }
        for d in bt {
            let battery = d.battery.map_or("sin batería conocida".to_string(), |b| format!("batería {b}%"));
            let name = if d.name.is_empty() { "(sin nombre)" } else { &d.name };
            writeln!(s, "  {name} · {battery}\n    {}", d.instance_id)?;
        }
        f.write_str(&s)
    }
}
