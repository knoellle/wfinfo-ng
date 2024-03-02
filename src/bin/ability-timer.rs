use std::{
    sync::mpsc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use clap::Parser;
use eframe::egui::{CentralPanel, Color32, Context, Frame};
use rdev::{EventType, Key};

fn main() {
    let arguments = Arguments::parse();
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Warframe Ability Timer",
        options,
        Box::new(|_cc| Box::<MyApp>::new(MyApp::new(arguments))),
    );
}

#[derive(Parser, Debug)]
/// Small tool to keep track of ESO ability timeouts in Warframe.
///
/// When one ability too often within a short time, Cephalon Simaris will disable that ability.
/// This tool should help using your abilities as often as possible without getting them disabled
/// by letting you know when you can safely use them again.
///
/// Example usage for Saryn's 4th: ability-timer 4 10
struct Arguments {
    /// Which ability key to look out for
    ability_key_number: u8,
    /// Minimum time between ability uses without angering Simaris
    timeout: f32,
}

struct MyApp {
    timeout: Duration,
    key: rdev::Key,

    last_activated: Option<Instant>,

    event_receiver: mpsc::Receiver<rdev::Event>,
    _listener: JoinHandle<()>,
}

impl MyApp {
    fn new(arguments: Arguments) -> Self {
        let key = match arguments.ability_key_number {
            1 => Key::Num1,
            2 => Key::Num2,
            3 => Key::Num3,
            4 => Key::Num4,
            x => panic!("Unexpected ability key: {x}, try 1-4"),
        };

        let (event_sender, event_receiver) = mpsc::channel();
        let listener = thread::spawn(move || {
            rdev::listen(move |event| event_sender.send(event).expect("failed to send event"))
                .expect("failed to set up listener");
        });

        Self {
            key,
            timeout: Duration::from_secs_f32(arguments.timeout),
            last_activated: Default::default(),
            event_receiver,
            _listener: listener,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        while let Ok(event) = self.event_receiver.try_recv() {
            if event.event_type == EventType::KeyPress(self.key) {
                self.last_activated = Some(Instant::now())
            }
        }

        let color = if self
            .last_activated
            .is_some_and(|last_activated| last_activated.elapsed() < self.timeout)
        {
            Color32::RED
        } else {
            Color32::GREEN
        };

        let frame = Frame::default().fill(color).inner_margin(4.0);
        CentralPanel::default().frame(frame).show(ctx, |_ui| {});
    }
}
