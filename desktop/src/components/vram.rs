use eframe::{egui::*, CreationContext};

const VRAM_WIDTH: usize = 1024;
const VRAM_HEIGHT: usize = 512;

fn convert_5bit_to_8bit(color: u16) -> u8 {
    (f64::from(color) * 255.0 / 31.0).round() as u8
}

pub struct VramViewer {
	vram_tex: TextureHandle
}

impl VramViewer {

	pub fn new(cc: &CreationContext) -> Self {
		Self {
			vram_tex: cc.egui_ctx.load_texture(
				"VRAM Viewer",
				ColorImage::new([VRAM_WIDTH, VRAM_HEIGHT], Color32::BLACK),
				TextureOptions::NEAREST
			)
		}
	}

	pub fn show(&mut self, ui: &mut Ui, psx: &psx::PSXEmulator) {

		let vram = psx.get_vram();
		let mut display_buf = vec![Color32::default(); VRAM_WIDTH * VRAM_HEIGHT];

		for y in 0..512 {
			for x in 0..1024 {
				let vram_addr = 1024 * y + x;
				let pixel = vram[vram_addr];

				if pixel != 0 {
					//println!("pixel: 0x{pixel:X}");
				}

				display_buf[x + VRAM_WIDTH * y] = Color32::from_rgb(
					convert_5bit_to_8bit((pixel >> 0) & 0x1F),
					convert_5bit_to_8bit((pixel >> 5) & 0x1F),
					convert_5bit_to_8bit((pixel >> 10) & 0x1F),
				)

			}
		}

		let colour_image = ColorImage {
			size: [VRAM_WIDTH, VRAM_HEIGHT],
			pixels: display_buf,
		};

		self.vram_tex.set(colour_image, TextureOptions::NEAREST);

		let image = Image::new(&self.vram_tex);

		ui.add(image);

	}

}