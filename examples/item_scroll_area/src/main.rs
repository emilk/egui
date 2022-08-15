#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

fn main() {
    let options = eframe::NativeOptions{
        min_window_size:Some(egui::vec2(750.,400.)),
        ..eframe::NativeOptions::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    );
}

struct MyApp {
    blocks: Vec<u32>
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            blocks:
            //vec![5,6,7/*,8,9,10,11,12,13,14,15,16,17*/],
            (0..=50).collect()
            //vec![10,50,7,100]
        }
    }
}

fn show_item_block(bi: usize, block: &mut u32, label_count: &mut usize, ui: &mut egui::Ui) {
    if ui.button(format!("add label to block {}", bi)).clicked() {
        *block += 1;
    }
    for i in (0..=*block).rev() {
        let text = format!("{}", i);
        egui::CollapsingHeader::new(&text)
            .default_open(true)
            .show(ui, |ui|{
                ui.label(text);
                *label_count += 1;
            });
        *label_count += 1;
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        let begin_scroll_area = std::time::Instant::now();
        let mut scroll_area_label_count = 0;
        egui::Window::new("ScrollArea").show(ctx, |ui|{
            if ui.button("add block").clicked() {
                self.blocks.push(0);
            }
            egui::containers::ScrollArea::vertical()
                .show(ui, |ui|{
                    for (bi, block) in self.blocks.iter_mut().enumerate() {
                        ui.push_id(bi, |ui|{
                            show_item_block(bi, block, &mut scroll_area_label_count, ui);
                        });
                    }
                });
        });
        let duration_scroll_area = begin_scroll_area.elapsed();

        let begin_item_scroll_area = std::time::Instant::now();
        let mut item_scroll_area_label_count = 0;
        egui::Window::new("ItemScrollArea").show(ctx, |ui|{
            if ui.button("add block").clicked() {
                self.blocks.push(0);
            }
            egui::containers::ItemScrollArea::vertical(self.blocks.len())
                .show_items(ui, |ui, bi|{
                    show_item_block(bi, &mut self.blocks[bi], &mut item_scroll_area_label_count, ui);
                });
        });
        let duration_item_scroll_area = begin_item_scroll_area.elapsed();

        let begin_scroll_area_blocks = std::time::Instant::now();
        let mut scroll_area_blocks_label_count = 0;
        egui::Window::new("show_blocks").show(ctx, |ui|{
            if ui.button("add block").clicked() {
                self.blocks.push(0);
            }
            egui::containers::ScrollArea::vertical()
                .show_blocks(self.blocks.len(), ui, |ui, bi|{
                    show_item_block(bi, &mut self.blocks[bi], &mut scroll_area_blocks_label_count, ui);
                });
        });
        let duration_scroll_area_blocks = begin_scroll_area_blocks.elapsed();

        egui::Window::new("compare").show(ctx, |ui|{
            egui::CollapsingHeader::new("ScrollArea")
                .default_open(true)
                .show(ui, |ui|{
                    ui.label(format!("render time: {:?}", duration_scroll_area));
                    ui.label(format!("label count: {}", scroll_area_label_count));
                });
            egui::CollapsingHeader::new("ItemScrollArea")
                .default_open(true)
                .show(ui, |ui|{
                    ui.label(format!("render time: {:?}", duration_item_scroll_area));
                    ui.label(format!("label count: {}", item_scroll_area_label_count));
                });
            egui::CollapsingHeader::new("ScrollArea.show_blocks")
                .default_open(true)
                .show(ui, |ui|{
                    ui.label(format!("render time: {:?}", duration_scroll_area_blocks));
                    ui.label(format!("label count: {}", scroll_area_blocks_label_count));
                });
        });
    }
}
