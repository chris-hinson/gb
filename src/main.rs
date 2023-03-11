use crate::cart::Cart;
use crate::cpu::Cpu;
use eframe::egui;
use egui::{ColorImage, TextureOptions};
use std::{
    alloc::System,
    sync::mpsc::{channel, Receiver, Sender},
    thread::JoinHandle,
    time::Duration,
};
use system::BackendCmd;

mod cart;
mod cpu;
mod io;
mod system;
fn main() -> Result<(), eframe::Error> {
    //println!("Hello, world!");

    let options = eframe::NativeOptions {
        ..Default::default()
    };
    eframe::run_native(
        "ap",
        options,
        Box::new(|cc| {
            let ctx = cc.egui_ctx.clone();

            Box::new(App::new(Box::new(move || {
                ctx.request_repaint();
            })))
        }),
    )
}

struct App {
    system_handle: Option<JoinHandle<()>>,
    log_channel: Receiver<String>,
    screen_channel: Receiver<Vec<u8>>,
    command_tx: Sender<BackendCmd>,
    command_rx: Receiver<FrontendCmd>,
    screen_tex: Option<egui::TextureHandle>,
    logs: Vec<String>,
}

impl App {
    fn new(repaint_frontend_callback: Box<dyn Fn() + Send>) -> Self {
        //intialize the system and start it in a new thread
        //TODO: write an emulator lol

        let (log_tx, log_rx) = channel();
        let (screen_tx, screen_rx) = channel();
        //for commands channels, they are named after where they are sendind TO, and where they are receiving AT
        let (front_cmd_tx, front_cmd_rx) = channel();
        let (back_cmd_tx, back_cmd_rx) = channel();

        let cpu = cpu::Cpu::new().unwrap();
        let cart = Cart::new(&mut std::fs::File::open("../roms/test_rom.gb").unwrap()).unwrap();
        let io = io::Io::new();
        let boot_room = include_bytes!("../dmg.bin").to_vec();

        let mut sys = system::System::new(
            log_tx,
            screen_tx,
            front_cmd_tx,
            back_cmd_rx,
            repaint_frontend_callback,
            cpu,
            cart,
            io,
            boot_room.try_into().unwrap(),
        );
        let system_handle = std::thread::spawn(move || sys.run());

        Self {
            system_handle: Some(system_handle),
            log_channel: log_rx,
            screen_channel: screen_rx,
            command_tx: back_cmd_tx,
            command_rx: front_cmd_rx,
            screen_tex: None,
            logs: Vec::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        //NOTE: this seems hacky. should it go somewhere else?
        //lock us to 60fps by saying that even if we dont get any repaint requests, force a repaint after 33.333ms
        ctx.request_repaint_after(Duration::from_nanos(33333));

        //nonblocking updates of backing data
        //get any pending logs
        let new_logs = self.log_channel.try_iter();
        for log in new_logs {
            println!("received");
            self.logs.push(log);

            println!("logs len {}", self.logs.len());
        }

        //get all of the screen updates we have been sent, and just display the last one
        let screen_data = self.screen_channel.try_iter();
        let l = screen_data.last();
        if l.is_some() && self.screen_tex.is_some() {
            self.screen_tex.as_mut().unwrap().set(
                ColorImage::from_rgb([160, 144], &l.unwrap()),
                TextureOptions::default(),
            );
        }

        //log area
        //-----------------------------------------------------------------------------------------
        egui::SidePanel::left("logs").show(ctx, |ui| {
            ui.heading("log_output");

            egui::ScrollArea::vertical().show(ui, |ui| {
                // Add a lot of widgets here.
                for (i, msg) in self.logs.iter().enumerate() {
                    ui.add_sized(
                        [ui.available_width(), 10.0],
                        egui::widgets::Label::new(format!("{}: {}", i, msg)),
                    );
                }
            });
        });
        //-----------------------------------------------------------------------------------------

        //screen area
        //-----------------------------------------------------------------------------------------
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("window1");
            let texture: &egui::TextureHandle = self.screen_tex.get_or_insert_with(|| {
                ui.ctx().load_texture(
                    "screen_image",
                    egui::ColorImage::example(),
                    Default::default(),
                )
            });
            ui.image(texture, texture.size_vec2());
        });
        //-----------------------------------------------------------------------------------------
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        //send a shutdown command to the backend
        self.command_tx.send(BackendCmd::Shutdown).unwrap();
        //wait for its thread handle to join before we kill our frontend
        //NOTE: hacky as fuck lmfao what
        if let Some(handle) = self.system_handle.take() {
            handle.join().expect("failed to join system thread");
        }
    }
}

#[derive(PartialEq)]
pub enum FrontendCmd {
    BackendDied,
    BreakpointHit,
    WatchpointHit,
}
