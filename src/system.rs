use crate::FrontendCmd;
use rand::prelude::*;
use rand::rngs::ThreadRng;
use std::sync::mpsc::{Receiver, Sender};
pub struct System {
    //(160x144)*3 = 69120
    screen_data: Vec<u8>,
    startup: std::time::Instant,
    log_tx: Sender<String>,
    screen_tx: Sender<Vec<u8>>,
    command_tx: Sender<FrontendCmd>,
    command_rx: Receiver<BackendCmd>,
    repaint_frontend_callback: Box<dyn Fn() + Send>,
}

impl System {
    pub fn new(
        log_tx: Sender<String>,
        screen_tx: Sender<Vec<u8>>,
        command_tx: Sender<FrontendCmd>,
        command_rx: Receiver<BackendCmd>,
        repaint_frontend_callback: Box<dyn Fn() + Send>,
    ) -> Self {
        Self {
            screen_data: vec![0; 69120],
            startup: std::time::Instant::now(),
            log_tx,
            screen_tx,
            command_tx,
            command_rx,
            repaint_frontend_callback,
        }
    }

    pub fn run(&mut self) {
        let mut last_sent = u64::MAX;
        let mut rand = rand::thread_rng();

        'running: loop {
            //see if we have gotten any commands from the frontend, and process and parse them if so
            let recv_cmds = self.command_rx.try_iter();
            for cmd in recv_cmds {
                if cmd == BackendCmd::Shutdown {
                    break 'running;
                }
            }

            //generate a log
            let cur = self.startup.elapsed().as_secs();
            if cur % 2 == 0 && ((cur / 2) != last_sent) {
                println!(
                    "sending {cur}, new last_sent is {}, which is not equal to {last_sent}",
                    cur / 5
                );
                self.log_tx.send(cur.to_string()).unwrap();
                last_sent = cur / 2;
                self.repaint_frontend_callback.as_mut()();
            }

            //scramble some pixels
            let x = rand.gen_range(0..160);
            let y = rand.gen_range(0..144);

            let r = rand.gen_range(0..0xff);
            let g = rand.gen_range(0..0xff);
            let b = rand.gen_range(0..0xff);

            let indx = (y * 3) * 160 + (x * 3);
            self.screen_data[indx] = r;
            self.screen_data[indx + 1] = g;
            self.screen_data[indx + 2] = b;

            self.screen_tx.send(self.screen_data.clone()).unwrap();
        }
    }
}

#[derive(PartialEq)]
pub enum BackendCmd {
    Shutdown,
}
