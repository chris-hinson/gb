use crate::cart::Cart;
use crate::cpu::Cpu;
use eframe::egui;
use egui::{ColorImage, TextureOptions};
use std::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::Duration,
};
use system::BackendCmd;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;

mod audio;
mod cart;
mod cpu;
mod io;
mod ppu;
mod system;
fn main() -> Result<(), eframe::Error> {
    pretty_env_logger::init();
    info!("starting up");

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
    system_mutex: Arc<Mutex<system::System>>,
    log_channel: Receiver<String>,
    screen_channel: Receiver<Vec<u8>>,
    command_tx: Sender<BackendCmd>,
    command_rx: Receiver<FrontendCmd>,
    cpu_rx: Receiver<Cpu>,
    screen_tex: Option<egui::TextureHandle>,
    logs: Vec<String>,
    cpu_state: Option<Cpu>,
    mem_editor: egui_memory_editor::MemoryEditor,
    dummy_memory: Vec<u8>,
    //memory_rx: Receiver<(usize, Vec<u8>)>,
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
        let (cpu_tx, cpu_rx) = channel();
        //let (mem_tx, mem_rx) = channel();

        let cpu = cpu::Cpu::new().unwrap();
        let cart = Cart::new(&mut std::fs::File::open("./roms/test_rom.gb").unwrap()).unwrap();
        let io = io::Io::new();
        let boot_room = include_bytes!("../dmg.bin").to_vec();
        println!("boot room is : {:x} bytes long", boot_room.len());

        let mut sys = system::System::new(
            log_tx,
            screen_tx,
            front_cmd_tx,
            back_cmd_rx,
            cpu_tx,
            repaint_frontend_callback,
            cpu,
            cart,
            io,
            boot_room.try_into().unwrap(),
            //mem_tx,
        );
        let big_ole_mutex = Arc::new(Mutex::new(sys));
        let sys_for_us = big_ole_mutex.clone();

        let thread_builder = std::thread::Builder::new().name("core".to_string());
        //let system_handle = std::thread::spawn(move || sys.run());
        let system_handle = thread_builder
            .spawn(move || crate::system::run_mutex(big_ole_mutex))
            .unwrap();

        Self {
            system_handle: Some(system_handle),
            system_mutex: sys_for_us,
            log_channel: log_rx,
            screen_channel: screen_rx,
            command_tx: back_cmd_tx,
            command_rx: front_cmd_rx,
            cpu_rx,
            screen_tex: None,
            logs: Vec::new(),
            cpu_state: None,
            mem_editor: egui_memory_editor::MemoryEditor::new()
                .with_window_title("memory")
                //.with_address_range("VRAM", 0x8000..0xA000),
                .with_address_range("ROM bank 0", 0..0x3FFF)
                //.with_address_range("ROM bank 1", 0..0x7FFF)
                .with_address_range("VRAM", 0x8000..0xA000),
            //.with_address_range("ExRAM", 0x)
            dummy_memory: vec![0; 0xFFFF],
            //memory_rx: mem_rx,
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
            //println!("{log}");
            self.logs.push(log);
            //debug!("{}", log)
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

        //get latest cpu state
        let cpu_state = self.cpu_rx.try_iter();
        let l = cpu_state.last();
        if l.is_some() {
            self.cpu_state = Some(l.unwrap());
        }

        //update all of our memory views
        let mut sys = self.system_mutex.lock().unwrap();
        unsafe {
            //copy in VRAM
            let src_ptr = sys.vram.as_ptr();
            let dst_ptr = self.dummy_memory.as_mut_ptr().add(0x8000);
            std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, 8192);

            //copy in BANK 0 (either bootroom or ROM bank 0)
            let bank = sys.read(0x0, 16384).unwrap();
            let src_ptr = bank.as_ptr();
            let dst_ptr = self.dummy_memory.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, bank.len());
        }
        drop(sys);

        //log area
        //-----------------------------------------------------------------------------------------
        /*egui::SidePanel::left("logs").show(ctx, |ui| {
            ui.heading("log_output");
            //ui.with_layout(egui::Layout::right_to_left(egui::Align::LEFT), |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Add a lot of widgets here.
                for (i, msg) in self.logs.iter().enumerate() {
                    ui.add_sized(
                        [ui.available_width(), 10.0],
                        egui::widgets::Label::new(format!("{}: {}", i, msg)),
                    );
                }
            });
            //});
        });*/
        egui::Window::new("logs").show(ctx, |ui| {
            ui.heading("logs");
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

        //cpu Area
        //-----------------------------------------------------------------------------------------
        //-----------------------------------------------------------------------------------------
        egui::SidePanel::right("cpu").show(ctx, |ui| {
            ui.heading("cpu state");
            ui.add_sized(
                [ui.available_width(), 10.0],
                egui::widgets::Label::new(format!("{}", self.cpu_state.clone().unwrap())),
            );
        });

        //screen area
        //-----------------------------------------------------------------------------------------
        /*egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("window1");
            let texture: &egui::TextureHandle = self.screen_tex.get_or_insert_with(|| {
                ui.ctx().load_texture(
                    "screen_image",
                    egui::ColorImage::example(),
                    Default::default(),
                )
            });
            ui.image(texture, texture.size_vec2());
        });*/
        egui::Window::new("screen").show(ctx, |ui| {
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

        //memory_editor
        //egui::Window::new("test").show(ctx, add_contents)
        /*egui::Window::new("mem_edit_test").show(ctx, |ui| {
            ui.add(
                self.mem_editor
                    .window_ui_read_only(ctx, is_open, mem, read_fn),
            )
        });*/
        /*egui::Window::new("mem_test_window").show(ctx, |ui| {
            ui.add({self.mem_editor.draw_editor_contents_read_only(
                ctx,
                &mut self.dummy_memory,
                |mem, address| mem.get(address).copied(),
            ))}
        });*/
        self.mem_editor.window_ui_read_only(
            ctx,
            &mut true,
            &mut self.dummy_memory,
            |mem, address| mem.get(address).copied(),
        );
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
