use hidapi::{HidApi, HidDevice};
use std::{thread, time};
use clap::Parser;
use colored::*;

/// UT61E+ USB multimeter reader
/// with help from https://github.com/ljakob/unit_ut61eplus/
#[derive(Parser)]
struct Args {
    /// Output as CSV
    #[arg(long)]
    csv: bool,
}

const DEVICE_IDS: &[(u16, u16)] = &[
    (0x1A86, 0xE429), // QinHeng
    (0x10C4, 0xEA80), // Silicon Labs CP2110
];

const GET_MEASUREMENT: [u8; 6] = [0xAB, 0xCD, 0x03, 0x5E, 0x01, 0xD9];

fn open_ut61eplus(api: &HidApi) -> Option<HidDevice> {
    for (vid, pid) in DEVICE_IDS {
        if let Ok(dev) = api.open(*vid, *pid) {
            println!("Opened UT61E+ with VID=0x{:04x}, PID=0x{:04x}", vid, pid);
            return Some(dev);
        }
    }
    None
}

fn send_command(dev: &HidDevice, cmd: &[u8]) -> Result<(), hidapi::HidError> {
    let mut buf = Vec::with_capacity(cmd.len() + 1);
    buf.push(cmd.len() as u8);
    buf.extend_from_slice(cmd);
    dev.write(&buf)?;
    Ok(())
}

fn read_response(dev: &HidDevice) -> Option<Vec<u8>> {
    let mut buf = [0u8; 64];
    loop {
        match dev.read(&mut buf) {
            Ok(n) if n > 0 => {
                // Skip first byte (length), look for 0xAB 0xCD header
                let data = &buf[1..n];
                if data.len() > 3 && data[0] == 0xAB && data[1] == 0xCD {
                    // Length is data[2], payload is data[3..]
                    let payload_len = data[2] as usize;
                    if data.len() >= 3 + payload_len {
                        // Drop last 2 bytes (checksum)
                        return Some(data[3..3 + payload_len - 2].to_vec());
                    }
                }
            }
            _ => return None,
        }
    }
}

fn parse_display_ascii(payload: &[u8]) -> String {
    // Digits are at payload[2..9] (see Python code)
    payload
        .get(2..9)
        .map(|slice| String::from_utf8_lossy(slice).replace(' ', ""))
        .unwrap_or_else(|| "?".to_string())
}

fn parse_mode(mode: u8) -> &'static str {
    match mode {
        0 => "V_AC",
        24 => "V_AC_LPF",
        2 => "V_DC",
        25 => "V_AC_DC",
        1 => "mV_AC",
        3 => "mV_DC",
        6 => "Resistance Ω",
        7 => "Continuity 🕪",
        8 => "Diode 𜰏",
        9 => "Capacitance 𜰓",
        4 => "Hz",
        5 => "%",
        18 => "Transistor gain 𜰐 β hFE",
        12 => "μA_DC",
        13 => "μA_AC",
        14 => "mA_DC",
        15 => "mA_AC",
        16 => "A_DC",
        17 => "A_AC",
        20 => "NCV",
        _ => "?",
    }
}

fn parse_unit(mode: u8, range: u8) -> &'static str {
    match mode {
        0 => match range { // VAC
            0x30 => "V",
            0x31 => "V",
            0x32 => "V",
            0x33 => "V",
            _ => "?",
        },
        24 => match range { // VAC LPF
            0x30 => "V",
            0x31 => "V",
            0x32 => "V",
            0x33 => "V",
            _ => "?",
        },
        2 => match range { // VDC
            0x30 => "V",
            0x31 => "V",
            0x32 => "V",
            0x33 => "V",
            _ => "?",
        },
        25 => match range { // VACDC
            0x30 => "V",
            0x31 => "V",
            0x32 => "V",
            0x33 => "V",
            _ => "?",
        },
        1 => match range { // mVAC
            0x30 => "mV",
            _ => "?",
        },
        3 => match range { // mVDC
            0x30 => "mV",
            _ => "?",
        },
        6 => match range { // Resistance
            0x30 => "Ω",
            0x31 => "kΩ",
            0x32 => "kΩ",
            0x33 => "kΩ",
            0x34 => "MΩ",
            0x35 => "MΩ",
            0x36 => "MΩ",
            _ => "?",
        },
        7 => match range { // Continuity
            0x30..=0x36 => "Ω",
            _ => "?",
        },
        8 => match range { // Diode
            0x30 => "V",
            _ => "?",
        },
        9 => match range { // Capacitance
            0x30 => "nF",
            0x31 => "nF",
            0x32 => "μF",
            0x33 => "μF",
            0x34 => "μF",
            0x35 => "mF",
            0x36 => "mF",
            _ => "?",
        },
        4 => match range { // Hz
            0x30 => "Hz",
            0x31 => "Hz",
            0x32 => "kHz",
            0x33 => "kHz",
            0x34 => "kHz",
            0x35 => "MHz",
            0x36 => "MHz",
            0x37 => "MHz",
            _ => "?",
        },
        5 => match range { // %
            0x30 => "%",
            _ => "?",
        },
        18 => match range { // hFE
            0x30 => "β",
            _ => "?",
        },
        12 => match range { // μA_DC
            0x30 => "μA",
            0x31 => "μA",
            _ => "?",
        },
        13 => match range { // μA_AC
            0x30 => "μA",
            0x31 => "μA",
            _ => "?",
        },
        14 => match range { // mA_DC
            0x30 => "mA",
            0x31 => "mA",
            _ => "?",
        },
        15 => match range { // mA_AC
            0x30 => "mA",
            0x31 => "mA",
            _ => "?",
        },
        16 => match range { // A_DC
            0x31 => "A",
            _ => "?",
        },
        17 => match range { // A_AC
            0x31 => "A",
            _ => "?",
        },
        20 => match range { // NCV
            0x30 => "NCV",
            _ => "?",
        },
        _ => "?",
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let api = HidApi::new()?;
    let dev =
        open_ut61eplus(&api).expect("UT61E+ device not found (tried all known VID/PID pairs)");

    // Enable UART, set baudrate, purge FIFOs
    // dev.send_feature_report(&[0x41, 0x01])?;
    // dev.send_feature_report(&[0x50, 0x00, 0x00, 0x25, 0x80, 0x00, 0x00, 0x03, 0x00, 0x00])?;
    // dev.send_feature_report(&[0x43, 0x02])?;

    if args.csv {
        println!("value,unit,mode,range,rel,hold,minmax");
    } else {
        println!("{}", "UT61E+ connected. Reading measurements...".bold().green());
    }

    loop {
        send_command(&dev, &GET_MEASUREMENT)?;
        if let Some(payload) = read_response(&dev) {
            let display = parse_display_ascii(&payload);
            let mode = payload.get(0).copied().unwrap_or(0);
            let range = payload.get(1).copied().unwrap_or(0);
            let unit = parse_unit(mode, range);
            let mode_str = parse_mode(mode);

            // Extract auto/manual status from second last byte
            let auto_manual_byte = payload.get(payload.len().saturating_sub(2)).copied().unwrap_or(0);
            let auto_manual = match auto_manual_byte {
                48 => "AUTO",
                52 => "MANUAL",
                _ => "?",
            };

            // Extract REL status from third last byte
            let flags_byte = payload.get(payload.len().saturating_sub(3)).copied().unwrap_or(0);

            // Bitwise flags
            let rel = if flags_byte & 0x01 != 0 { "REL" } else { "" };
            let hold = if flags_byte & 0x02 != 0 { "HOLD" } else { "" };
            let minmax = match flags_byte {
                56 => "MAX",
                52 => "MIN",
                _ => "",
            };

            if args.csv {
                println!("{},{},{},{},{},{},{}", display, unit, mode_str, auto_manual, rel, hold, minmax);
            } else {
                println!(
                    "{} {} {} {} {} {} {}",
                    display.bold().yellow(),
                    unit.cyan(),
                    format!("({})", mode_str).blue(),
                    format!("[{}]", auto_manual).magenta(),
                    rel.red(),
                    hold.red(),
                    minmax.red()
                );
            }
        } else {
            if !args.csv {
                println!("{}", "No response or parse error.".red());
            }
        }

        // UT61 display updates around 3 times
        // per second but this code is not particularly fast either and I'm not sure what the limit
        // is on the USB
        thread::sleep(time::Duration::from_millis(1000/6));
    }
    #[cfg(debug_assertions)]
    {
        let hex_string = payload.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");
        println!("DEBUG: payload hex: {}", hex_string);
    }
}
