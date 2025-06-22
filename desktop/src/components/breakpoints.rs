use std::fmt::Display;

use eframe::egui;
use egui_extras::{Column, TableBuilder};

use psx::PSXEmulator;

#[derive(PartialEq, Clone, Copy)]
enum BreakpointType {
	Exec,
	Read,
	Write
}

impl Display for BreakpointType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Exec => write!(f, "Exec"),
			Self::Read => write!(f, "Read"),
			Self::Write => write!(f, "Write"),
		}
	}
}

pub struct Breakpoint {
	address: u32,
	breakpoint_type: BreakpointType,
	label: String,
	active: bool,
	emu_index: usize,
}

impl Breakpoint {
	pub fn is_hit(&self, psx: &mut PSXEmulator) -> bool {
		match self.breakpoint_type {
			BreakpointType::Exec => {
				psx.cpu.pc == self.address
			},
			BreakpointType::Read | BreakpointType::Write => {
				psx.bus.breakpoint_hit.1 == self.address
			},
		}
	}
}


pub struct Breakpoints {
	pub breakpoints: Vec<Breakpoint>,

	text_address: String,
	parsed_addr: u32,
	text_label: String,
	combo_type: BreakpointType,

	remove_bp: (bool, usize)
}

impl Breakpoints {
	pub fn new() -> Self {
		Self {
			breakpoints: Vec::new(),

			text_address: String::from("0x00000000"),
			parsed_addr: 0,
			text_label: String::from("Breakpoint 0"),
			combo_type: BreakpointType::Exec,

			remove_bp: (false, 0),
		}
	}

	pub fn show(&mut self, ctx: &egui::Context, psx: &mut PSXEmulator, bp_open: &mut bool, new_bp_open: &mut bool) {
		egui::Window::new("Breakpoints").open(bp_open).show(ctx, |ui| {
			if ui.button("Add Breakpoint").clicked() {
				*new_bp_open = true;
			}

			ui.separator();

			let table = TableBuilder::new(ui)
				.column(Column::auto().at_least(80.0))
				.column(Column::auto().at_least(80.0))
				.column(Column::auto().at_least(40.0))
				.column(Column::auto().at_least(5.0))
				.column(Column::remainder())
				.striped(true);

			table.header(20.0, |mut header| {
				header.col(|ui| {
					ui.label("Label");
				});
				header.col(|ui| {
					ui.label("Address");
				});
				header.col(|ui| {
					ui.label("Type");
				});
				header.col(|ui| {
					ui.label("Active");
				});
				header.col(|ui| {
					ui.label("Remove");
				});
			})
			.body(|body| {
				body.rows(19.0, self.breakpoints.len(), |mut row| {
					let i = row.index();

					if psx.breakpoint_hit && self.breakpoints[i].is_hit(psx) {
						row.set_selected(true);
					}

					// label
					row.col(|ui| {
						ui.label(self.breakpoints[i].label.clone());
					});
					// address
					row.col(|ui| {
						ui.monospace(format!("0x{:08X}", self.breakpoints[i].address));
					});
					// breakpoint type
					row.col(|ui| {
						ui.label(format!("{}", self.breakpoints[i].breakpoint_type));
					});
					// active
					row.col(|ui| {
						if ui.checkbox(&mut self.breakpoints[i].active, "").changed() {
							// re-add breakpoint to emulator
							match self.breakpoints[i].active {
								true => {
									println!("re-enable breakpoint");
									let index = emu_add_breakpoint(&self.breakpoints[i], psx);

									self.breakpoints[i].emu_index = index;
								},
								// remove breakpoint from emulator
								false => {
									emu_remove_breakpoint(&self.breakpoints[i], psx);
								}
							};
						}
					});
					// remove
					row.col(|ui| {
						if ui.button("x").clicked() {
							// can't directly remove the breakpoint here otherwise the loop will have the wrong index
							self.remove_bp = (true, i);
						}
					});
				});
			});
		});

		// remove breakpoint from emulator and ui
		if self.remove_bp.0 {
			emu_remove_breakpoint(&self.breakpoints[self.remove_bp.1], psx);

			self.breakpoints.remove(self.remove_bp.1);
			self.remove_bp.0 = false;
		}

		egui::Window::new("Add Breakpoint").open(new_bp_open).show(ctx, |ui| {
			ui.horizontal(|ui| {
				ui.label("Address: ");

				let addr_text_edit = ui.text_edit_singleline(&mut self.text_address);
				
				if addr_text_edit.lost_focus() {
					self.text_address.retain(|c| c.is_ascii_hexdigit());
					self.parsed_addr = u32::from_str_radix(&self.text_address, 16).unwrap_or(4) & !0b11;
	
					self.text_address = format!("0x{:08X}", self.parsed_addr);
				}
			});

			ui.horizontal(|ui| {
				ui.label("Label: ");
				ui.text_edit_singleline(&mut self.text_label);
			});

			egui::ComboBox::from_label("Breakpoint Type")
				.selected_text(format!("{}", self.combo_type))
				.show_ui(ui, |ui| {
					ui.selectable_value(&mut self.combo_type, BreakpointType::Exec, "Exec");
					ui.selectable_value(&mut self.combo_type, BreakpointType::Read, "Read");
					ui.selectable_value(&mut self.combo_type, BreakpointType::Write, "Write");
				});
			
			if ui.button("Add").clicked() {

				let mut breakpoint = Breakpoint {
					address: self.parsed_addr,
					breakpoint_type: self.combo_type,
					label: self.text_label.clone(),
					active: true,
					emu_index: 0,
				};

				let index = emu_add_breakpoint(&breakpoint, psx);
				breakpoint.emu_index = index;

				self.breakpoints.push(breakpoint);

				// reset inputs fields to default values
				self.text_label = String::from(format!("Breakpoint {}", self.breakpoints.len()));
				self.text_address = String::from("0x00000000");
				self.combo_type = BreakpointType::Exec;
			}
		});
	}
}

fn emu_add_breakpoint(breakpoint: &Breakpoint, psx: &mut PSXEmulator) -> usize {
	match breakpoint.breakpoint_type {
		BreakpointType::Exec => {
			psx.pc_breakpoints.push(breakpoint.address);
			psx.pc_breakpoints.len() - 1
		},
		BreakpointType::Read => {
			psx.bus.read_breakpoints.push(breakpoint.address);
			psx.bus.read_breakpoints.len() - 1
		},
		BreakpointType::Write => {
			psx.bus.write_breakpoints.push(breakpoint.address);
			psx.bus.write_breakpoints.len() - 1
		}
	}
}

fn emu_remove_breakpoint(breakpoint: &Breakpoint, psx: &mut PSXEmulator) {
	match breakpoint.breakpoint_type {
		BreakpointType::Exec => {
			psx.pc_breakpoints.remove(breakpoint.emu_index);
		},
		BreakpointType::Read => {
			psx.bus.read_breakpoints.remove(breakpoint.emu_index);
		},
		BreakpointType::Write => {
			psx.bus.write_breakpoints.remove(breakpoint.emu_index);
		}
	}
}