// Move all UT61E+ logic to a library module
// This file will become the egui app entry point

use std::env;

fn main() {
    // match env::var("UT61E_SIM").as_deref() {
    //     Ok("gui") => ut61e_plus_logger::run_egui_app_simulated(),
    //     Ok("cli") => ut61e_plus_logger::run_cli_simulated(),
    //     _ => ut61e_plus_logger::run_egui_app(),
    // }
    ut61e_plus_logger::run_egui_app_simulated();
}
