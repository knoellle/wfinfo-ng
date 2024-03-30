use eframe::{egui::CentralPanel, App, NativeOptions};

use crate::ocr::OcrResult;

#[derive(Default)]
pub struct Overlay {
    frame: usize,
    position: (i32, i32),
    width: u32,
}

impl App for Overlay {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        self.frame += 1;
        // ctx.send_viewport_cmd(eframe::egui::ViewportCommand::center_on_screen(ctx).unwrap());
        if self.frame == 1 {
            // ctx.send_viewport_cmd(ViewportCommand::center_on_screen(ctx).unwrap());
            let f = ctx.native_pixels_per_point().unwrap();
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(
                [self.position.0 as f32 / f, self.position.1 as f32 / f].into(),
            ));
            // ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(
            //     [self.width as f32, 200.0].into(),
            // ));
        }

        if self.frame > 100 {
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
        }

        // if let Some(command) = ctx.input(|i| {
        //     let outer_rect = i.viewport().outer_rect?;
        //     let size = outer_rect.size();
        //     let monitor_size = i.viewport().monitor_size?;
        //     i.viewport().mo
        //     if 1.0 < monitor_size.x && 1.0 < monitor_size.y {
        //         let perc = (self.frame % 100) as f32 / 100.0;
        //         let x = (monitor_size.x) * dbg!(perc);
        //         let y = (monitor_size.y - size.y) / 2.0;
        //         Some(ViewportCommand::OuterPosition([x, y].into()))
        //     } else {
        //         println!("failed to construct command");
        //         None
        //     }
        // }) {
        //     ctx.send_viewport_cmd(command);
        // };

        if self.frame == 200 {
            let dimensions = ctx.input(|i| i.viewport().outer_rect);
            dbg!(self.width);
            dbg!(dimensions);
        }
        CentralPanel::default().show(ctx, |ui| {
            if ui.button("ayyy").clicked() {};
        });
    }
}

impl Overlay {
    pub fn show(ocr: OcrResult, window_position: (i32, i32), window_size: (i32, i32)) {
        let x = window_position.0 + window_size.0 / 2 - 150;
        let y = window_position.1 + window_size.1 / 2 - 150;
        let options = NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_always_on_top()
                // .with_mouse_passthrough(true)
                .with_transparent(true)
                .with_decorations(false)
                // .with_position([x as f32 / f, y as f32 / f])
                .with_resizable(false)
                .with_inner_size([300.0, 300.0]),
            ..Default::default()
        };
        eframe::run_native(
            "WFInfo Overlay",
            options,
            Box::new(move |_cc| {
                Box::<Overlay>::new(Overlay::new(ocr, window_position, window_size))
            }),
        )
        .unwrap();
    }

    pub fn new(ocr: OcrResult, window_position: (i32, i32), window_size: (i32, i32)) -> Self {
        let min_x = ocr
            .parts
            .iter()
            .map(|part| part.position.0)
            .min()
            .unwrap_or(50);
        let max_x = ocr
            .parts
            .iter()
            .map(|part| part.position.0 + part.image.width() / 2)
            .max()
            .unwrap_or(50);
        let y = ocr
            .parts
            .iter()
            .map(|part| part.position.1 + part.image.height())
            .max()
            .unwrap_or(500);

        dbg!(min_x);
        dbg!(max_x);
        dbg!(y);
        dbg!(window_position);
        dbg!(window_size);

        let x = window_position.0 + window_size.0 / 2 - 150;
        let y = window_position.1 + window_size.1 / 2 - 150;

        Self {
            frame: 0,
            position: dbg!(x, y),
            width: max_x - min_x,
        }
    }
}
