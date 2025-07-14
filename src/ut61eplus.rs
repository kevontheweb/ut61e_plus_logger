use eframe::{egui, App};
use egui_plot::{Plot, Line};
use hidapi::{HidApi, HidDevice};
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::{thread, time};

const DEVICE_IDS: &[(u16, u16)] = &[
    (0x1A86, 0xE429), // QinHeng
    (0x10C4, 0xEA80), // Silicon Labs CP2110
];

const GET_MEASUREMENT: [u8; 6] = [0xAB, 0xCD, 0x03, 0x5E, 0x01, 0xD9];

pub struct Ut61ePlus {
    dev: HidDevice,
}

#[derive(Debug, Clone)]
pub struct Measurement {
    pub value: f32,
    pub unit: String,
    pub mode: String,
    pub auto_manual: String,
    pub rel: String,
    pub hold: String,
    pub minmax: String,
}

#[derive(Debug)]
struct PlotApp {
    values: Arc<Mutex<Vec<f32>>>,
    measurement: Arc<Mutex<Measurement>>,
    ctx: Option<egui::Context>,
    selected_button: Option<usize>,
}

impl Ut61ePlus {
    pub fn open(api: &HidApi) -> Option<Self> {
        for (vid, pid) in DEVICE_IDS {
            if let Ok(dev) = api.open(*vid, *pid) {
                return Some(Self { dev });
            }
        }
        None
    }

    pub fn get_measurement(&self) -> Option<Measurement> {
        send_command(&self.dev, &GET_MEASUREMENT).ok()?;
        let payload = read_response(&self.dev)?;
        let display = parse_display_ascii(&payload);
        let value = display.parse::<f32>().ok()?;
        let mode = payload.get(0).copied().unwrap_or(0);
        let range = payload.get(1).copied().unwrap_or(0);
        let unit = parse_unit(mode, range).to_string();
        let mode_str = parse_mode(mode).to_string();
        let auto_manual_byte = payload
            .get(payload.len().saturating_sub(2))
            .copied()
            .unwrap_or(0);
        let auto_manual = match auto_manual_byte {
            48 => "AUTO",
            52 => "MANUAL",
            _ => "?",
        }
        .to_string();
        let flags_byte = payload
            .get(payload.len().saturating_sub(3))
            .copied()
            .unwrap_or(0);
        let rel = if flags_byte & 0x01 != 0 { "REL" } else { "" }.to_string();
        let hold = if flags_byte & 0x02 != 0 { "HOLD" } else { "" }.to_string();
        let minmax = match flags_byte {
            56 => "MAX",
            52 => "MIN",
            _ => "",
        }
        .to_string();
        Some(Measurement {
            value,
            unit,
            mode: mode_str,
            auto_manual,
            rel,
            hold,
            minmax,
        })
    }
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
                let data = &buf[1..n];
                if data.len() > 3 && data[0] == 0xAB && data[1] == 0xCD {
                    let payload_len = data[2] as usize;
                    if data.len() >= 3 + payload_len {
                        return Some(data[3..3 + payload_len - 2].to_vec());
                    }
                }
            }
            _ => return None,
        }
    }
}

fn parse_display_ascii(payload: &[u8]) -> String {
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
        0 => match range {
            0x30..=0x33 => "V",
            _ => "?",
        },
        24 => match range {
            0x30..=0x33 => "V",
            _ => "?",
        },
        2 => match range {
            0x30..=0x33 => "V",
            _ => "?",
        },
        25 => match range {
            0x30..=0x33 => "V",
            _ => "?",
        },
        1 => match range {
            0x30 => "mV",
            _ => "?",
        },
        3 => match range {
            0x30 => "mV",
            _ => "?",
        },
        6 => match range {
            0x30 => "Ω",
            0x31..=0x33 => "kΩ",
            0x34..=0x36 => "MΩ",
            _ => "?",
        },
        7 => match range {
            0x30..=0x36 => "Ω",
            _ => "?",
        },
        8 => match range {
            0x30 => "V",
            _ => "?",
        },
        9 => match range {
            0x30..=0x31 => "nF",
            0x32..=0x34 => "μF",
            0x35..=0x36 => "mF",
            _ => "?",
        },
        4 => match range {
            0x30..=0x31 => "Hz",
            0x32..=0x34 => "kHz",
            0x35..=0x37 => "MHz",
            _ => "?",
        },
        5 => match range {
            0x30 => "%",
            _ => "?",
        },
        18 => match range {
            0x30 => "β",
            _ => "?",
        },
        12 | 13 => match range {
            0x30..=0x31 => "μA",
            _ => "?",
        },
        14 | 15 => match range {
            0x30..=0x31 => "mA",
            _ => "?",
        },
        16 | 17 => match range {
            0x31 => "A",
            _ => "?",
        },
        20 => match range {
            0x30 => "NCV",
            _ => "?",
        },
        _ => "?",
    }
}

impl App for PlotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        if self.ctx.is_none() {
            self.ctx = Some(ctx.clone());
        }
        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            let m = self.measurement.lock().unwrap().clone();
            ui.heading("Meter Info");
            ui.label(format!("Mode: {}", m.mode));
            ui.label(format!("Unit: {}", m.unit));
            ui.label(format!("Range: {}", m.auto_manual));
            ui.label(format!("REL: {}", m.rel));
            ui.label(format!("HOLD: {}", m.hold));
            ui.label(format!("MIN/MAX: {}", m.minmax));
            ui.separator();
            ui.heading("Settings");
            let button_labels = ["Range", "REL", "Hold", "Min/Max"];
            ui.horizontal(|ui| {
                for (i, label) in button_labels.iter().enumerate() {
                    let mut button = egui::Button::new(*label);
                    if self.selected_button == Some(i) {
                        button = button.fill(ui.visuals().selection.bg_fill);
                    }
                    if ui.add(button).clicked() {
                        self.selected_button = Some(i);
                    }
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            let m = self.measurement.lock().unwrap().clone();
            ui.add_space(10.0);
            ui.heading(egui::RichText::new(format!("{:.4} {}", m.value, m.unit)).size(48.0));
            ui.add_space(10.0);
            let values = self.values.lock().unwrap();
            let points: Vec<[f64; 2]> = values
                .iter()
                .enumerate()
                .map(|(i, v)| [i as f64, *v as f64])
                .collect();
            Plot::new("plot").show(ui, |plot_ui| {
                plot_ui.line(Line::new("value", points));
            });
        });
    }
}

pub fn run_egui_app() {
    let api = HidApi::new().expect("Failed to open HID API");
    let dev = Ut61ePlus::open(&api).expect("UT61E+ device not found");
    let values = Arc::new(Mutex::new(Vec::new()));
    let measurement = Arc::new(Mutex::new(Measurement {
        value: 0.0,
        unit: "V".to_string(),
        mode: "V_AC".to_string(),
        auto_manual: "AUTO".to_string(),
        rel: "".to_string(),
        hold: "".to_string(),
        minmax: "".to_string(),
    }));
    let values_clone = values.clone();
    let measurement_clone = measurement.clone();
    std::thread::spawn(move || {
        loop {
            if let Some(val) = dev.get_measurement() {
                let mut buf = values_clone.lock().unwrap();
                buf.push(val.value);
                if buf.len() > 200 {
                    buf.remove(0);
                }
                let mut m = measurement_clone.lock().unwrap();
                *m = val;
            }
            thread::sleep(time::Duration::from_millis(1000 / 6));
        }
    });
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "UT61E+ Live Plot",
        native_options,
        Box::new(|_cc| Ok(Box::new(PlotApp { values, measurement, ctx: None, selected_button: None }) as Box<dyn App>)),
    );
}

pub fn run_cli() {
    let api = HidApi::new().expect("Failed to open HID API");
    let dev = Ut61ePlus::open(&api).expect("UT61E+ device not found");
    println!("value");
    loop {
        if let Some(val) = dev.get_measurement() {
            println!("{}", val.value);
        }
        thread::sleep(time::Duration::from_millis(1000 / 6));
    }
}

pub fn run_egui_app_simulated() {
    let values = Arc::new(Mutex::new(Vec::new()));
    let measurement = Arc::new(Mutex::new(Measurement {
        value: 0.0,
        unit: "V".to_string(),
        mode: "V_AC".to_string(),
        auto_manual: "AUTO".to_string(),
        rel: "".to_string(),
        hold: "".to_string(),
        minmax: "".to_string(),
    }));
    let values_clone = values.clone();
    let measurement_clone = measurement.clone();
    std::thread::spawn(move || {
        let mut t = 0.0f32;
        let mut rng = rand::rng();
        let modes= ["V_AC", "V_DC", "Resistance", "Hz", "%", "A_DC"];
        let units = ["V", "V", "Ω", "Hz", "%", "A"];
        let mut mode_idx = 0;
        loop {
            let simulated = (t * 0.2).sin() * 10.0 + rng.random_range(-0.5..0.5);
            t += 1.0;
            {
                let mut buf = values_clone.lock().unwrap();
                buf.push(simulated);
                if buf.len() > 200 {
                    buf.remove(0);
                }
                let mut m = measurement_clone.lock().unwrap();
                m.value = simulated;
                if (t as u32) % 100 == 0 {
                    mode_idx = (mode_idx + 1) % modes.len();
                    m.mode = modes[mode_idx].to_string();
                    m.unit = units[mode_idx].to_string();
                }
                m.auto_manual = if (t as u32) % 60 < 30 { "AUTO".to_string() } else { "MANUAL".to_string() };
                m.rel = if (t as u32) % 50 < 10 { "REL".to_string() } else { "".to_string() };
                m.hold = if (t as u32) % 80 < 10 { "HOLD".to_string() } else { "".to_string() };
                m.minmax = if (t as u32) % 120 < 10 { "MAX".to_string() } else if (t as u32) % 120 > 110 { "MIN".to_string() } else { "".to_string() };
            }
            thread::sleep(time::Duration::from_millis(1000 / 6));
        }
    });
    let native_options = eframe::NativeOptions::default();
    let app = PlotApp { values, measurement, ctx: None, selected_button: None };
    let _ = eframe::run_native(
        "UT61E+ Live Plot",
        native_options,
        Box::new(|_cc| Ok(Box::new(app) as Box<dyn App>)),
    );
}

pub fn run_cli_simulated() {
    println!("value");
    let mut t = 0.0f32;
    let mut rng = rand::rng();
    loop {
        let simulated = (t * 0.2).sin() * 10.0 + rng.random_range(-0.5..0.5);
        t += 1.0;
        println!("{}", simulated);
        thread::sleep(time::Duration::from_millis(1000 / 6));
    }
}
