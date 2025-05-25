use eframe::egui::{style::Style, text::LayoutJob, Color32, RichText, Ui};

use egui_extras::{Column, TableBuilder, TableRow};
use psx::cpu::instructions::*;
use psx::PSXEmulator;

const REG_NAMES: [&str; 32] = [
	"zero", "at",
	"v0", "v1",
	"a0", "a1", "a2", "a3",
	"t0", "t1", "t2", "t3", "t4", "t5", "t6", "t7",
	"s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7",
	"t8", "t9",
	"k0", "k1",
	"gp", "sp", "fp", "ra"
];

const DISASM_LINES: usize = 1024;
const LINE_ACTIVE: char = '>';
const LINE_BREAKPOINT: char = '⏺';

pub struct Disassembly {
	target_addr: u32,
	addr_text: String,
	follow_pc: bool,
}

impl Disassembly {
	pub fn new() -> Self {
		Self {
			target_addr: 0xBFC00000,
			addr_text: String::from("0xBFC00000"),
			follow_pc: true
		}
	}

	pub fn show(&mut self, ui: &mut Ui, psx: &mut PSXEmulator) {
		
		ui.strong("Disassembly");

		self.show_header(ui);

		ui.separator();

		let mut table = TableBuilder::new(ui)
			.column(Column::auto().at_least(1.0))
			.column(Column::auto().at_least(80.0))
			.column(Column::remainder())
			.striped(true);
		
		if self.follow_pc {
			self.target_addr = psx.cpu.pc;
			table = table.scroll_to_row(DISASM_LINES / 2, Some(eframe::egui::Align::Center));
		}

		let start_addr = self.target_addr.wrapping_sub((DISASM_LINES * 4) as u32 / 2);
		
		table
			.header(20.0, |mut header| {
				header.col(|ui| {
					ui.label(" ");
				});
				header.col(|ui| {
					ui.label("Address");
				});
				header.col(|ui| {
					ui.label("Instruction");
				});
			})
			.body(|body| {
				body.rows(12.0, DISASM_LINES, |mut row| {
					self.show_row(&mut row, start_addr, psx);
				});
			});

	}

	fn show_header(&mut self, ui: &mut Ui) {
		ui.horizontal(|ui| {
			ui.checkbox(&mut self.follow_pc, "Follow PC");

			ui.label("Jump to address: ");

			let addr_text_edit = ui.text_edit_singleline(&mut self.addr_text);
			
			if addr_text_edit.lost_focus() {
				self.addr_text.retain(|c| c.is_ascii_hexdigit());
				self.target_addr = u32::from_str_radix(&self.addr_text, 16).unwrap_or(0) & !0b11;

				self.addr_text = format!("0x{:08X}", self.target_addr);
			}
		});
	}

	fn show_row(&mut self, row: &mut TableRow, start_addr: u32, psx: &mut PSXEmulator) {
		let instr_addr = start_addr + (row.index() * 4) as u32;
		let instr = Instruction::from_u32(psx.bus.read32_debug(instr_addr));

		let mut status = RichText::new(' ');

		if instr_addr == psx.cpu.pc {
			row.set_selected(true);
			status = RichText::new(LINE_ACTIVE);
		}

		row.col(|ui| {
			ui.label(status);
		});

		row.col(|ui| {
			ui.monospace(format!("0x{instr_addr:08X}"));
		});

		row.col(|ui| {
			self.dissasemble_instr(instr, ui, psx);
		});
	}

	fn dissasemble_instr(&mut self, instr: Instruction, ui: &mut Ui, psx: &mut PSXEmulator) {
		let (mnemonic, fields) = instr.dissasemble();

		let mut desc = String::new();

		let style = Style::default();
		let mut disasm_line = LayoutJob::default();
		
		RichText::new(format!("{mnemonic} "))
			.monospace()
			.color(Color32::from_rgb(198, 120, 221))
			.append_to(&mut disasm_line, &style, eframe::egui::FontSelection::Default, eframe::egui::Align::Min);

		let mut first_field = true;

		for field in fields {

			if !first_field {
				RichText::new(", ")
					.monospace()
					.append_to(&mut disasm_line, &style, eframe::egui::FontSelection::Default, eframe::egui::Align::Min);
			} else {
				RichText::new(" ")
					.monospace()
					.append_to(&mut disasm_line, &style, eframe::egui::FontSelection::Default, eframe::egui::Align::Min);

				first_field = false;
			}

			match field {
				InstrField::Reg(reg) => {
					RichText::new("$")
						.monospace()
						.append_to(&mut disasm_line, &style, eframe::egui::FontSelection::Default, eframe::egui::Align::Min);

					RichText::new(format!("{}", REG_NAMES[reg as usize]))
						.monospace()
						.color(Color32::from_rgb(224, 108, 117))
						.append_to(&mut disasm_line, &style, eframe::egui::FontSelection::Default, eframe::egui::Align::Min);

					desc = format!("{desc}${} = 0x{:08X}\n", REG_NAMES[reg as usize], psx.cpu.registers.read_gpr(reg));

				},
				InstrField::Tgt(tgt) => {
					RichText::new(format!("0x{tgt:X}"))
						.monospace()
						.color(Color32::from_rgb(209, 154, 102))
						.append_to(&mut disasm_line, &style, eframe::egui::FontSelection::Default, eframe::egui::Align::Min);
				}
				InstrField::Imm(imm) => {
					RichText::new(format!("0x{imm:X}"))
						.monospace()
						.color(Color32::from_rgb(209, 154, 102))
						.append_to(&mut disasm_line, &style, eframe::egui::FontSelection::Default, eframe::egui::Align::Min);
				},
				InstrField::Shamt(shamt) => {
					RichText::new(format!("0x{shamt:X}"))
						.monospace()
						.color(Color32::from_rgb(209, 154, 102))
						.append_to(&mut disasm_line, &style, eframe::egui::FontSelection::Default, eframe::egui::Align::Min);
				},
				InstrField::Addr(offset, base) => {
					RichText::new(format!("0x{:X}", offset.wrapping_add(base)))
						.monospace()
						.color(Color32::from_rgb(209, 154, 102))
						.append_to(&mut disasm_line, &style, eframe::egui::FontSelection::Default, eframe::egui::Align::Min);
				},
			};
		}

		ui.label(disasm_line)
			.on_hover_text(desc.trim());
	}
}