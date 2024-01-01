use std::cell::{RefCell, RefMut};
use std::collections::{Bound, BTreeMap, HashMap};
use std::num::ParseIntError;
use std::ops::BitOr;
use std::rc::Rc;
use crate::FLAGS6502::B;
use std::fmt::{Debug, LowerHex, Write};
use minifb::{Key, KeyRepeat, Window, WindowOptions};

#[macro_use(concat_string)]
extern crate concat_string;

type RamArray = [u8; 64 * 1024];

struct Bus {
    ram: RamArray,
}

impl Bus {
    fn new() -> Self {
        return Bus {
            ram: [0; 64 * 1024],
        };
    }

    fn write(&mut self, addr: u16, data: u8) {
        if addr >= 0x0000 && addr <= 0xFFFF {
            self.ram[addr as usize] = data;
        }
    }

    fn read(&self, addr: u16, read_only: bool) -> u8 {
        if addr >= 0x0000 && addr <= 0xFFFF {
            // let v = self.ram.get(addr).expect("Failed to read value from array").collect();
            return self.ram[addr as usize];
        }

        return 0x00;
    }
}

#[derive(Debug)]
#[repr(u8)]
enum FLAGS6502 {
    C = (1 << 0),
    // Carry Bit
    Z = (1 << 1),
    // Zero
    I = (1 << 2),
    // Disable Interrupts
    D = (1 << 3),
    // Decimal Mode (unused in this implementation)
    B = (1 << 4),
    // Break
    U = (1 << 5),
    // Unused
    V = (1 << 6),
    // Overflow
    N = (1 << 7), // Negative
}

type OperateFn = fn(&mut cpu6502) -> u8;
type AddrModeFn = OperateFn;

struct INSTRUCTION {
    pub name: String,
    pub operate: OperateFn,
    pub addr_mode: AddrModeFn,
    pub cycles: u8,
}

struct cpu6502 {
    a: u8,
    // Accumulator Register
    x: u8,
    // X Register
    y: u8,
    // Y Register
    stkp: u8,
    // Stack Pointer (points to location on bus)
    pc: u16,
    // Program Counter
    status: u8,
    // Status Register
    fetched: u8,
    addr_abs: u16,
    addr_rel: u16,
    opcode: u8,
    cycles: u8,
    lookup: Vec<INSTRUCTION>,
    bus: Bus,
    clock_count: u32,
    temp: u16,
}

type cpu = cpu6502;

impl cpu6502 {
    fn new() -> Self {
        let lookup: Vec<INSTRUCTION> = vec![
            INSTRUCTION {
                name: "BRK".to_string(),
                operate: cpu::BRK,
                addr_mode: cpu::IMM,
                cycles: 7,
            },
            INSTRUCTION {
                name: "ORA".to_string(),
                operate: cpu::ORA,
                addr_mode: cpu::IZX,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 8,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 3,
            },
            INSTRUCTION {
                name: "ORA".to_string(),
                operate: cpu::ORA,
                addr_mode: cpu::ZP0,
                cycles: 3,
            },
            INSTRUCTION {
                name: "ASL".to_string(),
                operate: cpu::ASL,
                addr_mode: cpu::ZP0,
                cycles: 5,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 5,
            },
            INSTRUCTION {
                name: "PHP".to_string(),
                operate: cpu::PHP,
                addr_mode: cpu::IMP,
                cycles: 3,
            },
            INSTRUCTION {
                name: "ORA".to_string(),
                operate: cpu::ORA,
                addr_mode: cpu::IMM,
                cycles: 2,
            },
            INSTRUCTION {
                name: "ASL".to_string(),
                operate: cpu::ASL,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "ORA".to_string(),
                operate: cpu::ORA,
                addr_mode: cpu::ABS,
                cycles: 4,
            },
            INSTRUCTION {
                name: "ASL".to_string(),
                operate: cpu::ASL,
                addr_mode: cpu::ABS,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 6,
            },
            INSTRUCTION {
                name: "BPL".to_string(),
                operate: cpu::BPL,
                addr_mode: cpu::REL,
                cycles: 2,
            },
            INSTRUCTION {
                name: "ORA".to_string(),
                operate: cpu::ORA,
                addr_mode: cpu::IZY,
                cycles: 5,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 8,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "ORA".to_string(),
                operate: cpu::ORA,
                addr_mode: cpu::ZPX,
                cycles: 4,
            },
            INSTRUCTION {
                name: "ASL".to_string(),
                operate: cpu::ASL,
                addr_mode: cpu::ZPX,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 6,
            },
            INSTRUCTION {
                name: "CLC".to_string(),
                operate: cpu::CLC,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "ORA".to_string(),
                operate: cpu::ORA,
                addr_mode: cpu::ABY,
                cycles: 4,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 7,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "ORA".to_string(),
                operate: cpu::ORA,
                addr_mode: cpu::ABX,
                cycles: 4,
            },
            INSTRUCTION {
                name: "ASL".to_string(),
                operate: cpu::ASL,
                addr_mode: cpu::ABX,
                cycles: 7,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 7,
            },
            INSTRUCTION {
                name: "JSR".to_string(),
                operate: cpu::JSR,
                addr_mode: cpu::ABS,
                cycles: 6,
            },
            INSTRUCTION {
                name: "AND".to_string(),
                operate: cpu::AND,
                addr_mode: cpu::IZX,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 8,
            },
            INSTRUCTION {
                name: "BIT".to_string(),
                operate: cpu::BIT,
                addr_mode: cpu::ZP0,
                cycles: 3,
            },
            INSTRUCTION {
                name: "AND".to_string(),
                operate: cpu::AND,
                addr_mode: cpu::ZP0,
                cycles: 3,
            },
            INSTRUCTION {
                name: "ROL".to_string(),
                operate: cpu::ROL,
                addr_mode: cpu::ZP0,
                cycles: 5,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 5,
            },
            INSTRUCTION {
                name: "PLP".to_string(),
                operate: cpu::PLP,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "AND".to_string(),
                operate: cpu::AND,
                addr_mode: cpu::IMM,
                cycles: 2,
            },
            INSTRUCTION {
                name: "ROL".to_string(),
                operate: cpu::ROL,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "BIT".to_string(),
                operate: cpu::BIT,
                addr_mode: cpu::ABS,
                cycles: 4,
            },
            INSTRUCTION {
                name: "AND".to_string(),
                operate: cpu::AND,
                addr_mode: cpu::ABS,
                cycles: 4,
            },
            INSTRUCTION {
                name: "ROL".to_string(),
                operate: cpu::ROL,
                addr_mode: cpu::ABS,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 6,
            },
            INSTRUCTION {
                name: "BMI".to_string(),
                operate: cpu::BMI,
                addr_mode: cpu::REL,
                cycles: 2,
            },
            INSTRUCTION {
                name: "AND".to_string(),
                operate: cpu::AND,
                addr_mode: cpu::IZY,
                cycles: 5,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 8,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "AND".to_string(),
                operate: cpu::AND,
                addr_mode: cpu::ZPX,
                cycles: 4,
            },
            INSTRUCTION {
                name: "ROL".to_string(),
                operate: cpu::ROL,
                addr_mode: cpu::ZPX,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 6,
            },
            INSTRUCTION {
                name: "SEC".to_string(),
                operate: cpu::SEC,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "AND".to_string(),
                operate: cpu::AND,
                addr_mode: cpu::ABY,
                cycles: 4,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 7,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "AND".to_string(),
                operate: cpu::AND,
                addr_mode: cpu::ABX,
                cycles: 4,
            },
            INSTRUCTION {
                name: "ROL".to_string(),
                operate: cpu::ROL,
                addr_mode: cpu::ABX,
                cycles: 7,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 7,
            },
            INSTRUCTION {
                name: "RTI".to_string(),
                operate: cpu::RTI,
                addr_mode: cpu::IMP,
                cycles: 6,
            },
            INSTRUCTION {
                name: "EOR".to_string(),
                operate: cpu::EOR,
                addr_mode: cpu::IZX,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 8,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 3,
            },
            INSTRUCTION {
                name: "EOR".to_string(),
                operate: cpu::EOR,
                addr_mode: cpu::ZP0,
                cycles: 3,
            },
            INSTRUCTION {
                name: "LSR".to_string(),
                operate: cpu::LSR,
                addr_mode: cpu::ZP0,
                cycles: 5,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 5,
            },
            INSTRUCTION {
                name: "PHA".to_string(),
                operate: cpu::PHA,
                addr_mode: cpu::IMP,
                cycles: 3,
            },
            INSTRUCTION {
                name: "EOR".to_string(),
                operate: cpu::EOR,
                addr_mode: cpu::IMM,
                cycles: 2,
            },
            INSTRUCTION {
                name: "LSR".to_string(),
                operate: cpu::LSR,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "JMP".to_string(),
                operate: cpu::JMP,
                addr_mode: cpu::ABS,
                cycles: 3,
            },
            INSTRUCTION {
                name: "EOR".to_string(),
                operate: cpu::EOR,
                addr_mode: cpu::ABS,
                cycles: 4,
            },
            INSTRUCTION {
                name: "LSR".to_string(),
                operate: cpu::LSR,
                addr_mode: cpu::ABS,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 6,
            },
            INSTRUCTION {
                name: "BVC".to_string(),
                operate: cpu::BVC,
                addr_mode: cpu::REL,
                cycles: 2,
            },
            INSTRUCTION {
                name: "EOR".to_string(),
                operate: cpu::EOR,
                addr_mode: cpu::IZY,
                cycles: 5,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 8,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "EOR".to_string(),
                operate: cpu::EOR,
                addr_mode: cpu::ZPX,
                cycles: 4,
            },
            INSTRUCTION {
                name: "LSR".to_string(),
                operate: cpu::LSR,
                addr_mode: cpu::ZPX,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 6,
            },
            INSTRUCTION {
                name: "CLI".to_string(),
                operate: cpu::CLI,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "EOR".to_string(),
                operate: cpu::EOR,
                addr_mode: cpu::ABY,
                cycles: 4,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 7,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "EOR".to_string(),
                operate: cpu::EOR,
                addr_mode: cpu::ABX,
                cycles: 4,
            },
            INSTRUCTION {
                name: "LSR".to_string(),
                operate: cpu::LSR,
                addr_mode: cpu::ABX,
                cycles: 7,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 7,
            },
            INSTRUCTION {
                name: "RTS".to_string(),
                operate: cpu::RTS,
                addr_mode: cpu::IMP,
                cycles: 6,
            },
            INSTRUCTION {
                name: "ADC".to_string(),
                operate: cpu::ADC,
                addr_mode: cpu::IZX,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 8,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 3,
            },
            INSTRUCTION {
                name: "ADC".to_string(),
                operate: cpu::ADC,
                addr_mode: cpu::ZP0,
                cycles: 3,
            },
            INSTRUCTION {
                name: "ROR".to_string(),
                operate: cpu::ROR,
                addr_mode: cpu::ZP0,
                cycles: 5,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 5,
            },
            INSTRUCTION {
                name: "PLA".to_string(),
                operate: cpu::PLA,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "ADC".to_string(),
                operate: cpu::ADC,
                addr_mode: cpu::IMM,
                cycles: 2,
            },
            INSTRUCTION {
                name: "ROR".to_string(),
                operate: cpu::ROR,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "JMP".to_string(),
                operate: cpu::JMP,
                addr_mode: cpu::IND,
                cycles: 5,
            },
            INSTRUCTION {
                name: "ADC".to_string(),
                operate: cpu::ADC,
                addr_mode: cpu::ABS,
                cycles: 4,
            },
            INSTRUCTION {
                name: "ROR".to_string(),
                operate: cpu::ROR,
                addr_mode: cpu::ABS,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 6,
            },
            INSTRUCTION {
                name: "BVS".to_string(),
                operate: cpu::BVS,
                addr_mode: cpu::REL,
                cycles: 2,
            },
            INSTRUCTION {
                name: "ADC".to_string(),
                operate: cpu::ADC,
                addr_mode: cpu::IZY,
                cycles: 5,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 8,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "ADC".to_string(),
                operate: cpu::ADC,
                addr_mode: cpu::ZPX,
                cycles: 4,
            },
            INSTRUCTION {
                name: "ROR".to_string(),
                operate: cpu::ROR,
                addr_mode: cpu::ZPX,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 6,
            },
            INSTRUCTION {
                name: "SEI".to_string(),
                operate: cpu::SEI,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "ADC".to_string(),
                operate: cpu::ADC,
                addr_mode: cpu::ABY,
                cycles: 4,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 7,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "ADC".to_string(),
                operate: cpu::ADC,
                addr_mode: cpu::ABX,
                cycles: 4,
            },
            INSTRUCTION {
                name: "ROR".to_string(),
                operate: cpu::ROR,
                addr_mode: cpu::ABX,
                cycles: 7,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 7,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "STA".to_string(),
                operate: cpu::STA,
                addr_mode: cpu::IZX,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 6,
            },
            INSTRUCTION {
                name: "STY".to_string(),
                operate: cpu::STY,
                addr_mode: cpu::ZP0,
                cycles: 3,
            },
            INSTRUCTION {
                name: "STA".to_string(),
                operate: cpu::STA,
                addr_mode: cpu::ZP0,
                cycles: 3,
            },
            INSTRUCTION {
                name: "STX".to_string(),
                operate: cpu::STX,
                addr_mode: cpu::ZP0,
                cycles: 3,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 3,
            },
            INSTRUCTION {
                name: "DEY".to_string(),
                operate: cpu::DEY,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "TXA".to_string(),
                operate: cpu::TXA,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "STY".to_string(),
                operate: cpu::STY,
                addr_mode: cpu::ABS,
                cycles: 4,
            },
            INSTRUCTION {
                name: "STA".to_string(),
                operate: cpu::STA,
                addr_mode: cpu::ABS,
                cycles: 4,
            },
            INSTRUCTION {
                name: "STX".to_string(),
                operate: cpu::STX,
                addr_mode: cpu::ABS,
                cycles: 4,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "BCC".to_string(),
                operate: cpu::BCC,
                addr_mode: cpu::REL,
                cycles: 2,
            },
            INSTRUCTION {
                name: "STA".to_string(),
                operate: cpu::STA,
                addr_mode: cpu::IZY,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 6,
            },
            INSTRUCTION {
                name: "STY".to_string(),
                operate: cpu::STY,
                addr_mode: cpu::ZPX,
                cycles: 4,
            },
            INSTRUCTION {
                name: "STA".to_string(),
                operate: cpu::STA,
                addr_mode: cpu::ZPX,
                cycles: 4,
            },
            INSTRUCTION {
                name: "STX".to_string(),
                operate: cpu::STX,
                addr_mode: cpu::ZPY,
                cycles: 4,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "TYA".to_string(),
                operate: cpu::TYA,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "STA".to_string(),
                operate: cpu::STA,
                addr_mode: cpu::ABY,
                cycles: 5,
            },
            INSTRUCTION {
                name: "TXS".to_string(),
                operate: cpu::TXS,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 5,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 5,
            },
            INSTRUCTION {
                name: "STA".to_string(),
                operate: cpu::STA,
                addr_mode: cpu::ABX,
                cycles: 5,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 5,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 5,
            },
            INSTRUCTION {
                name: "LDY".to_string(),
                operate: cpu::LDY,
                addr_mode: cpu::IMM,
                cycles: 2,
            },
            INSTRUCTION {
                name: "LDA".to_string(),
                operate: cpu::LDA,
                addr_mode: cpu::IZX,
                cycles: 6,
            },
            INSTRUCTION {
                name: "LDX".to_string(),
                operate: cpu::LDX,
                addr_mode: cpu::IMM,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 6,
            },
            INSTRUCTION {
                name: "LDY".to_string(),
                operate: cpu::LDY,
                addr_mode: cpu::ZP0,
                cycles: 3,
            },
            INSTRUCTION {
                name: "LDA".to_string(),
                operate: cpu::LDA,
                addr_mode: cpu::ZP0,
                cycles: 3,
            },
            INSTRUCTION {
                name: "LDX".to_string(),
                operate: cpu::LDX,
                addr_mode: cpu::ZP0,
                cycles: 3,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 3,
            },
            INSTRUCTION {
                name: "TAY".to_string(),
                operate: cpu::TAY,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "LDA".to_string(),
                operate: cpu::LDA,
                addr_mode: cpu::IMM,
                cycles: 2,
            },
            INSTRUCTION {
                name: "TAX".to_string(),
                operate: cpu::TAX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "LDY".to_string(),
                operate: cpu::LDY,
                addr_mode: cpu::ABS,
                cycles: 4,
            },
            INSTRUCTION {
                name: "LDA".to_string(),
                operate: cpu::LDA,
                addr_mode: cpu::ABS,
                cycles: 4,
            },
            INSTRUCTION {
                name: "LDX".to_string(),
                operate: cpu::LDX,
                addr_mode: cpu::ABS,
                cycles: 4,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "BCS".to_string(),
                operate: cpu::BCS,
                addr_mode: cpu::REL,
                cycles: 2,
            },
            INSTRUCTION {
                name: "LDA".to_string(),
                operate: cpu::LDA,
                addr_mode: cpu::IZY,
                cycles: 5,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 5,
            },
            INSTRUCTION {
                name: "LDY".to_string(),
                operate: cpu::LDY,
                addr_mode: cpu::ZPX,
                cycles: 4,
            },
            INSTRUCTION {
                name: "LDA".to_string(),
                operate: cpu::LDA,
                addr_mode: cpu::ZPX,
                cycles: 4,
            },
            INSTRUCTION {
                name: "LDX".to_string(),
                operate: cpu::LDX,
                addr_mode: cpu::ZPY,
                cycles: 4,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "CLV".to_string(),
                operate: cpu::CLV,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "LDA".to_string(),
                operate: cpu::LDA,
                addr_mode: cpu::ABY,
                cycles: 4,
            },
            INSTRUCTION {
                name: "TSX".to_string(),
                operate: cpu::TSX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "LDY".to_string(),
                operate: cpu::LDY,
                addr_mode: cpu::ABX,
                cycles: 4,
            },
            INSTRUCTION {
                name: "LDA".to_string(),
                operate: cpu::LDA,
                addr_mode: cpu::ABX,
                cycles: 4,
            },
            INSTRUCTION {
                name: "LDX".to_string(),
                operate: cpu::LDX,
                addr_mode: cpu::ABY,
                cycles: 4,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "CPY".to_string(),
                operate: cpu::CPY,
                addr_mode: cpu::IMM,
                cycles: 2,
            },
            INSTRUCTION {
                name: "CMP".to_string(),
                operate: cpu::CMP,
                addr_mode: cpu::IZX,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 8,
            },
            INSTRUCTION {
                name: "CPY".to_string(),
                operate: cpu::CPY,
                addr_mode: cpu::ZP0,
                cycles: 3,
            },
            INSTRUCTION {
                name: "CMP".to_string(),
                operate: cpu::CMP,
                addr_mode: cpu::ZP0,
                cycles: 3,
            },
            INSTRUCTION {
                name: "DEC".to_string(),
                operate: cpu::DEC,
                addr_mode: cpu::ZP0,
                cycles: 5,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 5,
            },
            INSTRUCTION {
                name: "INY".to_string(),
                operate: cpu::INY,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "CMP".to_string(),
                operate: cpu::CMP,
                addr_mode: cpu::IMM,
                cycles: 2,
            },
            INSTRUCTION {
                name: "DEX".to_string(),
                operate: cpu::DEX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "CPY".to_string(),
                operate: cpu::CPY,
                addr_mode: cpu::ABS,
                cycles: 4,
            },
            INSTRUCTION {
                name: "CMP".to_string(),
                operate: cpu::CMP,
                addr_mode: cpu::ABS,
                cycles: 4,
            },
            INSTRUCTION {
                name: "DEC".to_string(),
                operate: cpu::DEC,
                addr_mode: cpu::ABS,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 6,
            },
            INSTRUCTION {
                name: "BNE".to_string(),
                operate: cpu::BNE,
                addr_mode: cpu::REL,
                cycles: 2,
            },
            INSTRUCTION {
                name: "CMP".to_string(),
                operate: cpu::CMP,
                addr_mode: cpu::IZY,
                cycles: 5,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 8,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "CMP".to_string(),
                operate: cpu::CMP,
                addr_mode: cpu::ZPX,
                cycles: 4,
            },
            INSTRUCTION {
                name: "DEC".to_string(),
                operate: cpu::DEC,
                addr_mode: cpu::ZPX,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 6,
            },
            INSTRUCTION {
                name: "CLD".to_string(),
                operate: cpu::CLD,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "CMP".to_string(),
                operate: cpu::CMP,
                addr_mode: cpu::ABY,
                cycles: 4,
            },
            INSTRUCTION {
                name: "NOP".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 7,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "CMP".to_string(),
                operate: cpu::CMP,
                addr_mode: cpu::ABX,
                cycles: 4,
            },
            INSTRUCTION {
                name: "DEC".to_string(),
                operate: cpu::DEC,
                addr_mode: cpu::ABX,
                cycles: 7,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 7,
            },
            INSTRUCTION {
                name: "CPX".to_string(),
                operate: cpu::CPX,
                addr_mode: cpu::IMM,
                cycles: 2,
            },
            INSTRUCTION {
                name: "SBC".to_string(),
                operate: cpu::SBC,
                addr_mode: cpu::IZX,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 8,
            },
            INSTRUCTION {
                name: "CPX".to_string(),
                operate: cpu::CPX,
                addr_mode: cpu::ZP0,
                cycles: 3,
            },
            INSTRUCTION {
                name: "SBC".to_string(),
                operate: cpu::SBC,
                addr_mode: cpu::ZP0,
                cycles: 3,
            },
            INSTRUCTION {
                name: "INC".to_string(),
                operate: cpu::INC,
                addr_mode: cpu::ZP0,
                cycles: 5,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 5,
            },
            INSTRUCTION {
                name: "INX".to_string(),
                operate: cpu::INX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "SBC".to_string(),
                operate: cpu::SBC,
                addr_mode: cpu::IMM,
                cycles: 2,
            },
            INSTRUCTION {
                name: "NOP".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::SBC,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "CPX".to_string(),
                operate: cpu::CPX,
                addr_mode: cpu::ABS,
                cycles: 4,
            },
            INSTRUCTION {
                name: "SBC".to_string(),
                operate: cpu::SBC,
                addr_mode: cpu::ABS,
                cycles: 4,
            },
            INSTRUCTION {
                name: "INC".to_string(),
                operate: cpu::INC,
                addr_mode: cpu::ABS,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 6,
            },
            INSTRUCTION {
                name: "BEQ".to_string(),
                operate: cpu::BEQ,
                addr_mode: cpu::REL,
                cycles: 2,
            },
            INSTRUCTION {
                name: "SBC".to_string(),
                operate: cpu::SBC,
                addr_mode: cpu::IZY,
                cycles: 5,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 8,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "SBC".to_string(),
                operate: cpu::SBC,
                addr_mode: cpu::ZPX,
                cycles: 4,
            },
            INSTRUCTION {
                name: "INC".to_string(),
                operate: cpu::INC,
                addr_mode: cpu::ZPX,
                cycles: 6,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 6,
            },
            INSTRUCTION {
                name: "SED".to_string(),
                operate: cpu::SED,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "SBC".to_string(),
                operate: cpu::SBC,
                addr_mode: cpu::ABY,
                cycles: 4,
            },
            INSTRUCTION {
                name: "NOP".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 2,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 7,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::NOP,
                addr_mode: cpu::IMP,
                cycles: 4,
            },
            INSTRUCTION {
                name: "SBC".to_string(),
                operate: cpu::SBC,
                addr_mode: cpu::ABX,
                cycles: 4,
            },
            INSTRUCTION {
                name: "INC".to_string(),
                operate: cpu::INC,
                addr_mode: cpu::ABX,
                cycles: 7,
            },
            INSTRUCTION {
                name: "???".to_string(),
                operate: cpu::XXX,
                addr_mode: cpu::IMP,
                cycles: 7,
            },
        ];

        return Self {
            a: 0,
            x: 0,
            y: 0,
            stkp: 0,
            pc: 0,
            status: 0,
            fetched: 0,
            addr_abs: 0,
            addr_rel: 0,
            opcode: 0,
            cycles: 0,
            lookup,
            bus: Bus::new(),
            clock_count: 0,
            temp: 0,
        };
    }

    fn get_flag(&self, f: FLAGS6502) -> u8 {
        let f = f as u8;
        if (self.status & f) > 0 {
            1
        } else {
            0
        }
    }

    fn set_flag(&mut self, f: FLAGS6502, v: bool) {
        if v {
            self.status |= f as u8
        } else {
            self.status &= !(f as u8)
        }
    }

    // Addressing Modes
    fn IMP(cpu: &mut cpu6502) -> u8 {
        cpu.fetched = cpu.a;
        0
    }
    fn IMM(cpu: &mut cpu6502) -> u8 {
        cpu.pc += 1u16;
        cpu.addr_abs = cpu.pc;
        0
    }
    fn ZP0(cpu: &mut cpu6502) -> u8 {
        cpu.addr_abs = cpu.read(cpu.pc) as u16;
        cpu.pc += 1;
        cpu.addr_abs &= 0x00FF;

        0
    }

    fn ZPX(cpu: &mut cpu6502) -> u8 {
        cpu.addr_abs = (cpu.read(cpu.pc) + cpu.x) as u16;
        cpu.pc += 1;
        cpu.addr_abs &= 0x00FF;

        return 0;
    }

    fn ZPY(cpu: &mut cpu6502) -> u8 {
        cpu.addr_abs = (cpu.read(cpu.pc) + cpu.y) as u16;
        cpu.pc += 1;
        cpu.addr_abs &= 0x00FF;

        0
    }
    fn REL(cpu: &mut cpu6502) -> u8 {
        cpu.addr_rel = cpu.read(cpu.pc) as u16;
        cpu.pc += 1;
        if cpu.addr_rel & 0x80 != 0 {
            cpu.addr_rel |= 0xFF00;
        }
        0
    }


    fn ABS(cpu: &mut cpu6502) -> u8 {
        let lo = cpu.read(cpu.pc) as u16;
        cpu.pc += 1;
        let hi = cpu.read(cpu.pc) as u16;
        cpu.pc += 1;

        cpu.addr_abs = ((hi << 8) | lo) as u16;

        0
    }


    fn ABX(cpu: &mut cpu6502) -> u8 {
        let lo = cpu.read(cpu.pc) as u16;
        cpu.pc += 1;
        let hi = cpu.read(cpu.pc) as u16;
        cpu.pc += 1;

        cpu.addr_abs = ((hi << 8) | lo) as u16;
        cpu.addr_abs += cpu.x as u16;

        if (cpu.addr_abs & 0xFF00) != (hi << 8) as u16 {
            1
        } else {
            0
        }
    }


    fn ABY(cpu: &mut cpu6502) -> u8 {
        let lo = cpu.read(cpu.pc) as u16;
        cpu.pc += 1;
        let hi = cpu.read(cpu.pc) as u16;
        cpu.pc += 1;

        cpu.addr_abs = ((hi << 8) | lo);
        cpu.addr_abs += cpu.y as u16;

        if (cpu.addr_abs & 0xFF00) != (hi << 8) {
            1
        } else {
            0
        }
    }


    fn IND(cpu: &mut cpu6502) -> u8 {
        let ptr_lo = cpu.read(cpu.pc) as u16;
        cpu.pc += 1;
        let ptr_hi = cpu.read(cpu.pc) as u16;
        cpu.pc += 1;

        let ptr = (ptr_hi << 8) | ptr_lo;

        if ptr_lo == 0x00FF
        // Simulate page boundary hardware bug
        {
            cpu.addr_abs = (cpu.read(ptr & 0xFFu16) as u16) << 8 | (cpu.read(ptr + 0) as u16);
        } else
        // Behave normally
        {
            cpu.addr_abs = ((cpu.read(ptr + 1) as u16) << 8) | (cpu.read(ptr + 0) as u16);
        }

        0
    }


    fn IZX(cpu: &mut cpu6502) -> u8 {
        let t = cpu.read(cpu.pc) as u16;
        cpu.pc += 1;

        let lo = cpu.read(((t + (cpu.x as u16)) & 0x00FF)) as u16;
        let hi = cpu.read(((t + ((cpu.x as u16) + 1u16)) & 0x00FF)) as u16;

        cpu.addr_abs = ((hi << 8) | lo) as u16;

        0
    }


    fn IZY(cpu: &mut cpu6502) -> u8 {
        let t = cpu.read(cpu.pc) as u16;
        cpu.pc += 1;

        let lo = cpu.read((t & 0x00FF)) as u16;
        let hi = cpu.read(((t + 1) & 0x00FF)) as u16;

        cpu.addr_abs = ((hi << 8) | lo);
        cpu.addr_abs += cpu.y as u16;

        if (cpu.addr_abs & 0xFF00) != (hi << 8) {
            1
        } else {
            0
        }
    }

    //opcodes
    fn ADC(cpu: &mut cpu6502) -> u8 {
        // Grab the data that we are adding to the accumulator
        cpu.fetch();

        // Add is performed in 16-bit domain for emulation to capture any
        // carry bit, which will exist in bit 8 of the 16-bit word
        cpu.temp = ((cpu.a as u16) + (cpu.fetched as u16) + (cpu.get_flag(FLAGS6502::C) as u16));

        // The carry flag out exists in the high byte bit 0
        cpu.set_flag(FLAGS6502::C, cpu.temp > 255);

        // The Zero flag is set if the result is 0
        cpu.set_flag(FLAGS6502::Z, (cpu.temp & 0x00FF) == 0);

        // The signed Overflow flag is set based on all that up there! :D
        cpu.set_flag(
            FLAGS6502::V,
            (!((cpu.a as u16) ^ (cpu.fetched as u16)) & ((cpu.a as u16) ^ (cpu.temp as u16))) & 0x0080 != 0,
        );

        // The negative flag is set to the most significant bit of the result
        //Tawanda verify this
        cpu.set_flag(FLAGS6502::N, cpu.temp & 0x80 != 0);

        // Load the result into the accumulator (it's 8-bit dont forget!)
        cpu.a = (cpu.temp & 0x00FF) as u8;

        // This instruction has the potential to require an additional clock cycle
        return 1;
    }

    fn AND(cpu: &mut cpu6502) -> u8 {
        cpu.fetch();
        cpu.a = cpu.a & cpu.fetched;
        cpu.set_flag(FLAGS6502::Z, cpu.a == 0x00);
        cpu.set_flag(FLAGS6502::N, cpu.a & 0x80 != 0);
        return 1;
    }
    fn ASL(cpu: &mut cpu6502) -> u8 {
        cpu.fetch();
        cpu.temp = ((cpu.fetched as u16) << 1);
        cpu.set_flag(FLAGS6502::C, (cpu.temp & 0xFF00) > 0);
        cpu.set_flag(FLAGS6502::Z, (cpu.temp & 0x00FF) == 0x00);
        cpu.set_flag(FLAGS6502::N, cpu.temp & 0x80 != 0);
        if cpu.lookup[cpu.opcode as usize].addr_mode == cpu6502::IMP {
            cpu.a = (cpu.temp & 0x00FF) as u8;
        } else {
            cpu.write(cpu.addr_abs, (cpu.temp & 0x00FF) as u8);
        }

        return 0;
    }
    fn BCC(cpu: &mut cpu6502) -> u8 {
        if cpu.get_flag(FLAGS6502::C) == 0 {
            cpu.cycles += 1;
            cpu.addr_abs = cpu.pc + cpu.addr_rel;

            if (cpu.addr_abs & 0xFF00) != (cpu.pc & 0xFF00) {
                cpu.cycles += 1;
            }

            cpu.pc = cpu.addr_abs;
        }
        return 0;
    }
    fn BCS(cpu: &mut cpu6502) -> u8 {
        if cpu.get_flag(FLAGS6502::C) == 1 {
            cpu.cycles += 1;
            cpu.addr_abs = cpu.pc + cpu.addr_rel;

            if ((cpu.addr_abs & 0xFF00) != (cpu.pc & 0xFF00)) {
                cpu.cycles += 1;
            }

            cpu.pc = cpu.addr_abs;
        }
        return 0;
    }
    fn BEQ(cpu: &mut cpu6502) -> u8 {
        if cpu.get_flag(FLAGS6502::Z) == 1 {
            cpu.cycles += 1;
            cpu.addr_abs = cpu.pc + cpu.addr_rel;

            if (cpu.addr_abs & 0xFF00) != (cpu.pc & 0xFF00) {
                cpu.cycles += 1;
            }

            cpu.pc = cpu.addr_abs;
        }
        0
    }
    fn BIT(cpu: &mut cpu6502) -> u8 {
        cpu.fetch();
        cpu.temp = (cpu.a & cpu.fetched) as u16;
        cpu.set_flag(FLAGS6502::Z, (cpu.temp & 0x00FF) == 0x00);
        cpu.set_flag(FLAGS6502::N, cpu.fetched & (1 << 7) != 0);
        cpu.set_flag(FLAGS6502::V, cpu.fetched & (1 << 6) != 0);

        0
    }

    fn BMI(cpu: &mut cpu6502) -> u8 {
        if cpu.get_flag(FLAGS6502::N) == 1 {
            cpu.cycles += 1;
            cpu.addr_abs = cpu.pc + cpu.addr_rel;

            if (cpu.addr_abs & 0xFF00) != (cpu.pc & 0xFF00) {
                cpu.cycles += 1;
            }

            cpu.pc = cpu.addr_abs;
        }
        return 0;
    }

    fn BNE(cpu: &mut cpu6502) -> u8 {
        if cpu.get_flag(FLAGS6502::Z) == 0 {
            cpu.cycles += 1;
            cpu.addr_abs = cpu.pc + cpu.addr_rel;

            if (cpu.addr_abs & 0xFF00) != (cpu.pc & 0xFF00) {
                cpu.cycles += 1;
            }

            cpu.pc = cpu.addr_abs;
        }

        0
    }

    fn BPL(cpu: &mut cpu6502) -> u8 {
        if cpu.get_flag(FLAGS6502::N) == 0 {
            cpu.cycles += 1;
            cpu.addr_abs = cpu.pc + cpu.addr_rel;

            if (cpu.addr_abs & 0xFF00) != (cpu.pc & 0xFF00) {
                cpu.cycles += 1;
            }

            cpu.pc = cpu.addr_abs;
        }

        0
    }


    fn BRK(cpu: &mut cpu6502) -> u8 {
        cpu.pc += 1;

        cpu.set_flag(FLAGS6502::I, true);
        cpu.write(0x0100 + cpu.stkp as u16, ((cpu.pc >> 8) & 0x00FF) as u8);
        cpu.stkp -= 1;
        cpu.write(0x0100 + cpu.stkp as u16, (cpu.pc & 0x00FF) as u8);
        cpu.stkp -= 1;

        cpu.set_flag(FLAGS6502::B, true);
        cpu.write(0x0100 + cpu.stkp as u16, cpu.status);
        cpu.stkp -= 1;
        cpu.set_flag(FLAGS6502::B, false);

        cpu.pc = (cpu.read(0xFFFE) as u16) | ((cpu.read(0xFFFF) as u16) << 8);

        0
    }

    fn BVC(cpu: &mut cpu6502) -> u8 {
        if cpu.get_flag(FLAGS6502::V) == 0
        {
            cpu.cycles += 1;
            cpu.addr_abs = cpu.pc + cpu.addr_rel;

            if (cpu.addr_abs & 0xFF00) != (cpu.pc & 0xFF00) {
                cpu.cycles += 1;
            }


            cpu.pc = cpu.addr_abs;
        }

        0
    }


    fn BVS(cpu: &mut cpu6502) -> u8 {
        if cpu.get_flag(FLAGS6502::V) == 1
        {
            cpu.cycles += 1;
            cpu.addr_abs = cpu.pc + cpu.addr_rel;

            if (cpu.addr_abs & 0xFF00) != (cpu.pc & 0xFF00) {
                cpu.cycles += 1;
            }


            cpu.pc = cpu.addr_abs;
        }


        0
    }


    fn CLC(cpu: &mut cpu6502) -> u8 {
        cpu.set_flag(FLAGS6502::C, false);

        0
    }


    fn CLD(cpu: &mut cpu6502) -> u8 {
        cpu.set_flag(FLAGS6502::D, false);

        0
    }

    fn CLI(cpu: &mut cpu6502) -> u8 {
        cpu.set_flag(FLAGS6502::I, false);
        0
    }

    fn CLV(cpu: &mut cpu6502) -> u8 {
        cpu.set_flag(FLAGS6502::V, false);

        0
    }

    fn CMP(cpu: &mut cpu6502) -> u8 {
        cpu.fetch();
        cpu.temp = (cpu.a - cpu.fetched) as u16;
        cpu.set_flag(FLAGS6502::C, cpu.a >= cpu.fetched);
        cpu.set_flag(FLAGS6502::Z, (cpu.temp & 0x00FF) == 0x0000);
        cpu.set_flag(FLAGS6502::N, (cpu.temp & 0x0080) != 0);

        0
    }


    fn CPX(cpu: &mut cpu6502) -> u8 {
        cpu.fetch();
        cpu.temp = (cpu.x - cpu.fetched) as u16;
        cpu.set_flag(FLAGS6502::C, cpu.x >= cpu.fetched);
        cpu.set_flag(FLAGS6502::Z, (cpu.temp & 0x00FF) == 0x0000);
        cpu.set_flag(FLAGS6502::N, (cpu.temp & 0x0080) != 0);

        0
    }

    fn CPY(cpu: &mut cpu6502) -> u8 {
        cpu.fetch();
        cpu.temp = (cpu.y - cpu.fetched) as u16;
        cpu.set_flag(FLAGS6502::C, cpu.y >= cpu.fetched);
        cpu.set_flag(FLAGS6502::Z, (cpu.temp & 0x00FF) == 0x0000);
        cpu.set_flag(FLAGS6502::N, (cpu.temp & 0x0080) != 0);

        0
    }

    fn DEC(cpu: &mut cpu6502) -> u8 {
        cpu.fetch();
        cpu.temp = (cpu.fetched - 1) as u16;
        cpu.write(cpu.addr_abs, (cpu.temp & 0x00FF) as u8);
        cpu.set_flag(FLAGS6502::Z, (cpu.temp & 0x00FF) == 0x0000);
        cpu.set_flag(FLAGS6502::N, (cpu.temp & 0x0080) != 0);

        0
    }

    fn DEX(cpu: &mut cpu6502) -> u8 {
        cpu.x -= 1;
        cpu.set_flag(FLAGS6502::Z, cpu.x == 0x00);
        cpu.set_flag(FLAGS6502::N, (cpu.x & 0x80) != 0);

        0
    }


    fn DEY(cpu: &mut cpu6502) -> u8 {
        cpu.y -= 1;
        cpu.set_flag(FLAGS6502::Z, cpu.y == 0x00);
        cpu.set_flag(FLAGS6502::N, (cpu.y & 0x80) != 0);

        0
    }


    fn EOR(cpu: &mut cpu6502) -> u8 {
        cpu.fetch();
        cpu.a = cpu.a ^ cpu.fetched;

        cpu.set_flag(FLAGS6502::Z, cpu.a == 0x00);
        cpu.set_flag(FLAGS6502::N, (cpu.a & 0x80) != 0);

        0
    }


    fn INC(cpu: &mut cpu6502) -> u8 {
        cpu.fetch();
        cpu.temp = (cpu.fetched + 1) as u16;
        cpu.write(cpu.addr_abs, (cpu.temp & 0x00FF) as u8);
        cpu.set_flag(FLAGS6502::Z, (cpu.temp & 0x00FF) == 0x0000);
        cpu.set_flag(FLAGS6502::N, (cpu.temp & 0x0080) != 0);

        0
    }


    fn INX(cpu: &mut cpu6502) -> u8 {
        cpu.x += 1;

        cpu.set_flag(FLAGS6502::Z, cpu.x == 0x00);
        cpu.set_flag(FLAGS6502::N, (cpu.x & 0x80) != 0);

        0
    }


    fn INY(cpu: &mut cpu6502) -> u8 {
        cpu.y += 1;

        cpu.set_flag(FLAGS6502::Z, cpu.y == 0x00);
        cpu.set_flag(FLAGS6502::N, (cpu.y & 0x80) != 0);

        0
    }

    fn JMP(cpu: &mut cpu6502) -> u8 {
        cpu.pc = cpu.addr_abs;

        0
    }

    fn JSR(cpu: &mut cpu6502) -> u8 {
        cpu.pc -= 1;

        cpu.write(0x0100u16 + (cpu.stkp as u16), ((cpu.pc >> 8) & 0x00FF) as u8);
        cpu.stkp -= 1;
        cpu.write(0x0100u16 + (cpu.stkp as u16), (cpu.pc & 0x00FF) as u8);
        cpu.stkp -= 1;

        cpu.pc = cpu.addr_abs;

        0
    }


    fn LDA(cpu: &mut cpu6502) -> u8 {
        cpu.fetch();
        cpu.a = cpu.fetched;
        cpu.set_flag(FLAGS6502::Z, cpu.a == 0x00);
        cpu.set_flag(FLAGS6502::N, (cpu.a & 0x80) != 0);

        1
    }
    fn LDX(cpu: &mut cpu6502) -> u8 {
        cpu.fetch();
        cpu.x = cpu.fetched;
        cpu.set_flag(FLAGS6502::Z, cpu.x == 0x00);
        cpu.set_flag(FLAGS6502::N, (cpu.x & 0x80) != 0);


        1
    }
    fn LDY(cpu: &mut cpu6502) -> u8 {
        cpu.fetch();
        cpu.y = cpu.fetched;
        cpu.set_flag(FLAGS6502::Z, cpu.y == 0x00);
        cpu.set_flag(FLAGS6502::N, (cpu.y & 0x80) != 0);

        1
    }
    fn LSR(cpu: &mut cpu6502) -> u8 {
        cpu.fetch();
        cpu.set_flag(FLAGS6502::C, (cpu.fetched & 0x0001) != 0);
        cpu.temp = (cpu.fetched >> 1) as u16;
        cpu.set_flag(FLAGS6502::Z, (cpu.temp & 0x00FF) == 0x0000);
        cpu.set_flag(FLAGS6502::N, (cpu.temp & 0x0080) != 0);


        if cpu.lookup[cpu.opcode as usize].addr_mode == cpu6502::IMP {
            cpu.a = (cpu.temp & 0x00FF) as u8;
        } else {
            cpu.write(cpu.addr_abs, (cpu.temp & 0x00FF) as u8);
        }

        0
    }

    fn NOP(cpu: &mut cpu6502) -> u8 {
        let return_code = match cpu.opcode {
            0x1C => { 1 }
            0x3C => { 1 }
            0x5C => { 1 }
            0x7C => { 1 }
            0xDC => { 1 }
            0xFC => { 1 }
            _ => { 0 }
        };

        return_code
    }

    fn ORA(cpu: &mut cpu6502) -> u8 {
        cpu.fetch();
        cpu.a = cpu.a | cpu.fetched;
        cpu.set_flag(FLAGS6502::Z, cpu.a == 0x00);
        cpu.set_flag(FLAGS6502::N, (cpu.a & 0x80) != 0);

        1
    }
    fn PHA(cpu: &mut cpu6502) -> u8 {
        cpu.write(0x0100u16 + (cpu.stkp as u16), cpu.a);
        cpu.stkp -= 1;

        0
    }
    fn PHP(cpu: &mut cpu6502) -> u8 {
        cpu.write(0x0100u16 + (cpu.stkp as u16), cpu.status | (FLAGS6502::B as u8) | (FLAGS6502::U as u8));
        cpu.set_flag(FLAGS6502::B, false);
        cpu.set_flag(FLAGS6502::U, false);
        cpu.stkp -= 1;

        0
    }
    fn PLA(cpu: &mut cpu6502) -> u8 {
        cpu.stkp += 1;
        cpu.a = cpu.read(0x0100u16 + cpu.stkp as u16);
        cpu.set_flag(FLAGS6502::Z, cpu.a == 0x00);
        cpu.set_flag(FLAGS6502::N, (cpu.a & 0x80) != 0);

        0
    }

    fn PLP(cpu: &mut cpu6502) -> u8 {
        cpu.stkp += 1;
        cpu.status = cpu.read(0x0100u16 + cpu.stkp as u16);
        cpu.set_flag(FLAGS6502::U, true);


        0
    }

    fn ROL(cpu: &mut cpu6502) -> u8 {
        cpu.fetch();
        cpu.temp = ((cpu.get_flag(FLAGS6502::C) << 7) | (cpu.fetched >> 1)) as u16;
        cpu.set_flag(FLAGS6502::C, (cpu.fetched & 0x01) != 0);
        cpu.set_flag(FLAGS6502::Z, (cpu.temp & 0x00FF) == 0x00);
        cpu.set_flag(FLAGS6502::N, (cpu.temp & 0x0080) != 0);


        if cpu.lookup[cpu.opcode as usize].addr_mode == cpu6502::IMP {
            cpu.a = (cpu.temp & 0x00FF) as u8;
        } else {
            cpu.write(cpu.addr_abs, (cpu.temp & 0x00FF) as u8);
        }


        0
    }
    fn ROR(cpu: &mut cpu6502) -> u8 {
        cpu.fetch();
        cpu.temp = ((cpu.get_flag(FLAGS6502::C) << 7) | (cpu.fetched >> 1)) as u16;
        cpu.set_flag(FLAGS6502::C, (cpu.fetched & 0x01) != 0);
        cpu.set_flag(FLAGS6502::Z, (cpu.temp & 0x00FF) == 0x00);
        cpu.set_flag(FLAGS6502::N, (cpu.temp & 0x0080) != 0);


        if cpu.lookup[cpu.opcode as usize].addr_mode == cpu6502::IMP {
            cpu.a = (cpu.temp & 0x00FF) as u8;
        } else {
            cpu.write(cpu.addr_abs, (cpu.temp & 0x00FF) as u8);
        }

        0
    }


    fn RTI(cpu: &mut cpu6502) -> u8 {
        cpu.stkp += 1;
        cpu.status = cpu.read(0x0100u16 + cpu.stkp as u16);
        cpu.status &= !(FLAGS6502::B as u8);
        cpu.status &= !(FLAGS6502::U as u8);

        cpu.stkp += 1;
        cpu.pc = cpu.read(0x0100u16 + cpu.stkp as u16) as u16;
        cpu.stkp += 1;
        cpu.pc |= (cpu.read(0x0100u16 + cpu.stkp as u16) as u16) << 8;

        0
    }


    fn RTS(cpu: &mut cpu6502) -> u8 {
        cpu.stkp += 1;
        cpu.pc = cpu.read(0x0100u16 + cpu.stkp as u16) as u16;
        cpu.stkp += 1;
        cpu.pc |= (cpu.read(0x0100u16 + cpu.stkp as u16) as u16) << 8;

        cpu.pc += 1;

        0
    }
    fn SBC(cpu: &mut cpu6502) -> u8 {
        cpu.fetch();

        // Operating in 16-bit domain to capture carry out

        // We can invert the bottom 8 bits with bitwise xor
        let value = (cpu.fetched as u16) ^ 0x00FF;

        // Notice this is exactly the same as addition from here!
        cpu.temp = ((cpu.a as u16) + value + (cpu.get_flag(FLAGS6502::C) as u16));
        cpu.set_flag(FLAGS6502::C, cpu.temp & 0xFF00 != 0);
        cpu.set_flag(FLAGS6502::Z, ((cpu.temp & 0x00FF) == 0));
        cpu.set_flag(FLAGS6502::V, ((cpu.temp ^ (cpu.a as u16)) & (cpu.temp ^ (value)) & 0x0080) != 0);
        cpu.set_flag(FLAGS6502::N, (cpu.temp & 0x0080) != 0);
        cpu.a = (cpu.temp & 0x00FF) as u8;

        1
    }
    fn SEC(cpu: &mut cpu6502) -> u8 {
        cpu.set_flag(FLAGS6502::C, true);

        0
    }
    fn SED(cpu: &mut cpu6502) -> u8 {
        cpu.set_flag(FLAGS6502::D, true);

        0
    }
    fn SEI(cpu: &mut cpu6502) -> u8 {
        cpu.set_flag(FLAGS6502::I, true);

        0
    }

    fn STA(cpu: &mut cpu6502) -> u8 {
        cpu.write(cpu.addr_abs, cpu.a);

        0
    }

    fn STX(cpu: &mut cpu6502) -> u8 {
        cpu.write(cpu.addr_abs, cpu.x);

        0
    }
    fn STY(cpu: &mut cpu6502) -> u8 {
        cpu.write(cpu.addr_abs, cpu.y);

        0
    }
    fn TAX(cpu: &mut cpu6502) -> u8 {
        cpu.x = cpu.a;

        cpu.set_flag(FLAGS6502::Z, cpu.x == 0x00);
        cpu.set_flag(FLAGS6502::N, (cpu.x & 0x80) != 0);

        0
    }
    fn TAY(cpu: &mut cpu6502) -> u8 {
        cpu.y = cpu.a;

        cpu.set_flag(FLAGS6502::Z, cpu.y == 0x00);
        cpu.set_flag(FLAGS6502::N, (cpu.y & 0x80) != 0);

        0
    }
    fn TSX(cpu: &mut cpu6502) -> u8 {
        cpu.x = cpu.stkp;

        cpu.set_flag(FLAGS6502::Z, cpu.x == 0x00);
        cpu.set_flag(FLAGS6502::N, (cpu.x & 0x80) != 0);

        0
    }


    fn TXA(cpu: &mut cpu6502) -> u8 {
        cpu.a = cpu.x;

        cpu.set_flag(FLAGS6502::Z, cpu.a == 0x00);
        cpu.set_flag(FLAGS6502::N, (cpu.a & 0x80) != 0);

        0
    }


    fn TXS(cpu: &mut cpu6502) -> u8 {
        cpu.stkp = cpu.x;

        0
    }


    fn TYA(cpu: &mut cpu6502) -> u8 {
        cpu.a = cpu.y;

        cpu.set_flag(FLAGS6502::Z, cpu.a == 0x00);
        cpu.set_flag(FLAGS6502::N, (cpu.a & 0x80) != 0);

        0
    }

    // I capture all "unofficial" opcodes with this function. It is
    // functionally identical to a NOP
    fn XXX(cpu: &mut cpu6502) -> u8 {
        0
    }

    fn clock(&mut self) {
        if self.cycles == 0 {
            self.opcode = self.read(self.pc);


            println!("{}", self.lookup[self.opcode as usize].name);


            // Always set the unused status flag bit to 1
            self.set_flag(FLAGS6502::U, true);

            // Increment program counter, we read the opcode byte
            self.pc += 1;

            // Get Starting number of cycles
            self.cycles = self.lookup[self.opcode as usize].cycles;

            // Perform fetch of intermmediate data using the
            // required addressing mode
            let additional_cycle1 = (self.lookup[self.opcode as usize].addr_mode)(self);

            // Perform operation
            let additional_cycle2 = (self.lookup[self.opcode as usize].operate)(self);

            // The addressmode and opcode may have altered the number
            // of cycles this instruction requires before its completed
            self.cycles += (additional_cycle1 & additional_cycle2);

            // Always set the unused status flag bit to 1
            self.set_flag(FLAGS6502::U, true);

            println!("Value: {:02x}", self.read(self.addr_abs));
        }

        // Increment global clock count - This is actually unused unless logging is enabled
        // but I've kept it in because its a handy watch variable for debugging
        self.clock_count += 1;

        // Decrement the number of cycles remaining for this instruction
        self.cycles -= 1;
    }

    fn read(&mut self, address: u16) -> u8 {
        self.bus.read(address, false)
    }

    fn write(&mut self, address: u16, value: u8) {
        self.bus.write(address, value)
    }


    fn reset(&mut self) {
        // Get address to set program counter to
        self.addr_abs = 0xFFFC;


        let lo = self.read(self.addr_abs + 0) as u16;
        let hi = self.read(self.addr_abs + 1) as u16;

        println!("lo: {}, hi: {}", lo, hi);

        // Set it
        self.pc = ((hi << 8) | lo);

        println!("pc: {}", self.pc);

        // Reset internal registers
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.stkp = 0xFD;
        self.status = 0x00 | (FLAGS6502::U as u8);

        // Clear internal helper variables
        self.addr_rel = 0x0000;
        self.addr_abs = 0x0000;
        self.fetched = 0x00;

        // Reset takes time
        self.cycles = 8;
    }


    fn irq(&mut self) {
        if (self.get_flag(FLAGS6502::I) == 0) {
            // Push the program counter to the stack. It's 16-bits dont
            // forget so that takes two pushes
            self.write(
                (0x0100u16 + self.stkp as u16),
                ((self.pc >> 8) & 0x00FF) as u8,
            );
            self.stkp -= 1;
            self.write((0x0100u16 + self.stkp as u16), (self.pc & 0x00FF) as u8);
            self.stkp -= 1;

            // Then Push the status register to the stack
            self.set_flag(FLAGS6502::B, false);
            self.set_flag(FLAGS6502::U, true);
            self.set_flag(FLAGS6502::I, true);
            self.write(0x0100u16 + self.stkp as u16, self.status);
            self.stkp -= 1;

            // Read new program counter location from fixed address
            self.addr_abs = 0xFFFE;
            let lo = self.read(self.addr_abs + 0) as u16;
            let hi = self.read(self.addr_abs + 1) as u16;
            self.pc = ((hi << 8u16) | lo) as u16;

            // IRQs take time
            self.cycles = 7;
        }
    }

    //  #[allow(arithmetic_overflow)]
    fn nmi(&mut self) {
        self.write(
            0x0100u16 + self.stkp as u16,
            ((self.pc >> 8) & 0x00FF) as u8,
        );
        self.stkp -= 1;
        self.write(0x0100u16 + self.stkp as u16, (self.pc & 0x00FF) as u8);
        self.stkp -= 1;

        self.set_flag(FLAGS6502::B, false);
        self.set_flag(FLAGS6502::U, true);
        self.set_flag(FLAGS6502::I, true);
        self.write(0x0100u16 + self.stkp as u16, self.status);
        self.stkp -= 1;

        self.addr_abs = 0xFFFA;
        let lo = self.read(self.addr_abs + 0) as u16;
        let hi = self.read(self.addr_abs + 1) as u16;
        self.pc = ((hi << 8) | lo) as u16;

        self.cycles = 8;
    }

    fn fetch(&mut self) -> u8 {
        if !(self.lookup[self.opcode as usize].addr_mode == cpu::IMP) {
            self.fetched = self.read(self.addr_abs - 1);
        }

        return self.fetched;
    }

    fn complete(&mut self) -> bool {
        self.cycles == 0
    }

    fn connect_bus(&mut self, bus: Bus) {
        self.bus = bus
    }


    fn disassemble(&mut self, start: u16, stop: u16) -> BTreeMap<u16, String> {
        let mut addr = start;
        let mut value = 0x00u8;
        let mut lo = 0x00u8;
        let mut hi = 0x00u8;

        let mut line_addr = 0u16;

        let mut map_lines: BTreeMap<u16, String> = BTreeMap::new();

        while (addr as u32) <= 0xFFFF {
            line_addr = addr;

            let mut addr_hex = std::format!("${:04x}: ", addr);

            let opcode = self.bus.read(addr, true) as usize;
            addr += 1;

            addr_hex.push_str(std::format!("{} ", self.lookup[opcode].name).as_str());

            if self.lookup[opcode].addr_mode == cpu::IMP
            {
                addr_hex.push_str(" {IMP}");
            } else if self.lookup[opcode].addr_mode == cpu::IMM
            {
                value = self.bus.read(addr, true);
                addr += 1;

                addr_hex.push_str(std::format!("#${:02x} {}", value, "{IMM}").as_str());
            } else if self.lookup[opcode].addr_mode == cpu::ZP0
            {
                lo = self.bus.read(addr, true);
                addr += 1;
                hi = 0x00;
                addr_hex.push_str(std::format!("${:02x} {}", lo, "{ZP0}").as_str());
            } else if self.lookup[opcode].addr_mode == cpu::ZPX
            {
                lo = self.bus.read(addr, true);
                addr += 1;
                hi = 0x00;
                addr_hex.push_str(std::format!("${:02x} {}", lo, "{ZPX}").as_str());
            } else if self.lookup[opcode].addr_mode == cpu::ZPY
            {
                lo = self.bus.read(addr, true);
                addr += 1;
                hi = 0x00;
                addr_hex.push_str(std::format!("${:02x}, Y {}", lo, "{ZPY}").as_str());
            } else if self.lookup[opcode].addr_mode == cpu::IZX
            {
                lo = self.bus.read(addr, true);
                addr += 1;
                hi = 0x00;
                addr_hex.push_str(std::format!("(${:02x}, X) {}", lo, "{IZX}").as_str());
            } else if self.lookup[opcode].addr_mode == cpu::IZY
            {
                lo = self.bus.read(addr, true);
                addr += 1;
                hi = 0x00;
                addr_hex.push_str(std::format!("(${:02x}, Y) {}", lo, "{IZY}").as_str());
            } else if self.lookup[opcode].addr_mode == cpu::ABS
            {
                lo = self.bus.read(addr, true);
                addr += 1;
                hi = self.bus.read(addr, true);
                addr += 1;
                addr_hex.push_str(std::format!("${:04x} {}", ((hi as u16) << 8) | (lo as u16), "{ABS}").as_str());
            } else if self.lookup[opcode].addr_mode == cpu::ABX
            {
                lo = self.bus.read(addr, true);
                addr += 1;
                hi = self.bus.read(addr, true);
                addr += 1;
                addr_hex.push_str(std::format!("${:04x}, X {}", (((hi as u16) << 8) as u16) | (lo as u16), "{ABX}").as_str());
            } else if self.lookup[opcode].addr_mode == cpu::ABY
            {
                lo = self.bus.read(addr, true);
                addr += 1;
                hi = self.bus.read(addr, true);
                addr += 1;
                addr_hex.push_str(std::format!("${:04x}, Y {}", (((hi as u16) << 8) as u16) | (lo as u16), "{ABY}").as_str());
            } else if self.lookup[opcode].addr_mode == cpu::IND
            {
                lo = self.bus.read(addr, true);
                addr += 1;
                hi = self.bus.read(addr, true);
                addr += 1;
                addr_hex.push_str(std::format!("$({:04x}) {}", ((hi as u16) << 8) | (lo as u16), "{IND}").as_str());
            } else if self.lookup[opcode].addr_mode == cpu::REL
            {
                value = self.bus.read(addr, true);
                addr += 1;

                addr_hex.push_str(std::format!("$[{:04x}] {}", (addr + (value as u16)), "{REL}").as_str());
            }

            if addr == (0xFFFF - 1) {
                break;
            }

            // Add the formed string to a std::map, using the instruction's
            // address as the key. This makes it convenient to look for later
            // as the instructions are variable in length, so a straight up
            // incremental index is not sufficient.

            map_lines.insert(line_addr, addr_hex);
        }


        return map_lines;
    }
}


pub fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

pub fn encode_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}

fn to_hex<T: LowerHex>(number: T, d: u16) -> String {
    let mut s = String::new();

    if d == 2 {
        write!(&mut s, "{:02x}", number).unwrap();
    }

    if d == 4 {
        write!(&mut s, "{:04x}", number).unwrap();
    }

    s
}

fn print_cpu(cpu: &mut cpu6502)
{
    println!("pc: {:02x}", cpu.pc);
    println!("Acc register: {:02x} [{}]", cpu.a, cpu.a);
    println!("X register: {:02x} [{}]", cpu.x, cpu.x);
    println!("Y register: {:02x} [{}]", cpu.y, cpu.y);
    println!("Status Register: {:02x} [{}] [{:b}]", cpu.status, cpu.status, cpu.status);
    println!("Stack Pointer: {:02x}", cpu.stkp);
    println!("cycles: {:02x}", cpu.cycles);
    println!("fetched: {}", cpu.fetched);
    println!("Cycles comeplete: {:?}", cpu.complete());
}

const WIDTH: usize = 800;
const HEIGHT: usize = 600;

fn draw_cpu(status: &StatusText, cpu: &cpu6502, screen: &mut Vec<u32>, x: u32, y: u32) {
    status.draw(screen, (x as usize, y as usize), "STATUS: ", 1);


    status.draw(screen, ((x + 64) as usize, (y) as usize), "N", if cpu.status & (FLAGS6502::N as u8) != 0 { 0xFF00FFFF } else { 0xFF0000FF });
    status.draw(screen, ((x + 80) as usize, (y) as usize), "V", if cpu.status & (FLAGS6502::V as u8) != 0 { 0xFF00FFFF } else { 0xFF0000FF });
    status.draw(screen, ((x + 96) as usize, (y) as usize), "-", if cpu.status & (FLAGS6502::U as u8) != 0 { 0xFF00FFFF } else { 0xFF0000FF });
    status.draw(screen, ((x + 112) as usize, (y) as usize), "B", if cpu.status & (FLAGS6502::B as u8) != 0 { 0xFF00FFFF } else { 0xFF0000FF });
    status.draw(screen, ((x + 128) as usize, (y) as usize), "D", if cpu.status & (FLAGS6502::D as u8) != 0 { 0xFF00FFFF } else { 0xFF0000FF });
    status.draw(screen, ((x + 144) as usize, (y) as usize), "I", if cpu.status & (FLAGS6502::I as u8) != 0 { 0xFF00FFFF } else { 0xFF0000FF });
    status.draw(screen, ((x + 160) as usize, (y) as usize), "Z", if cpu.status & (FLAGS6502::Z as u8) != 0 { 0xFF00FFFF } else { 0xFF0000FF });
    status.draw(screen, ((x + 178) as usize, (y) as usize), "C", if cpu.status & (FLAGS6502::C as u8) != 0 { 0xFF00FFFF } else { 0xFF0000FF });

    status.draw(screen, (x as usize, (y + 10) as usize), std::format!("PC: ${:04x}", cpu.pc).as_str(), 1);
    status.draw(screen, (x as usize, (y + 20) as usize), std::format!("A : ${:02x}", cpu.a).as_str(), 1);
    status.draw(screen, (x as usize, (y + 30) as usize), std::format!("X : ${:02x}", cpu.x).as_str(), 1);
    status.draw(screen, (x as usize, (y + 40) as usize), std::format!("Y : ${:02x}", cpu.y).as_str(), 1);
    status.draw(screen, (x as usize, (y + 50) as usize), std::format!("Stack P: ${:#04x}", cpu.stkp).as_str(), 1);
}

fn draw_ram(status: &StatusText, cpu: &cpu6502, screen: &mut Vec<u32>, x: u32, y: u32, addr: u16, rows: u32, columns: u32)
{
    let mut ram_x = x as usize;
    let mut ram_y = y as usize;
    let mut naddr = addr;


    for row in 0..rows {
        let mut offset = std::format!("${:04x}:", naddr);

        for column in 0..columns {
            offset.push_str(std::format!(" {:02x}", cpu.bus.read(naddr, true)).as_str());

            naddr += 1;
        }

        status.draw(screen, (ram_x, ram_y), offset.as_str(), 1);
        ram_y += 10;
    }
}

fn draw_code(status: &StatusText, cpu: &cpu6502, screen: &mut Vec<u32>, x: u32, y: u32, lines: u32, map_lines: &mut BTreeMap<u16, String>) {

    let mut line_y = (lines >> 1) * 10 + y;




    if let Some(instruction) = map_lines.get(&cpu.pc) {
        status.draw(screen, (x as usize, line_y as usize), instruction, 0x00FF00FF);

        let mut it = map_lines.range_mut((Bound::Excluded(&cpu.pc), Bound::Unbounded));

        while line_y < (lines * 10) + y {
            line_y += 10;

            if let Some(next_asm) = &it.next() {
                status.draw(screen, (x as usize, line_y as usize), next_asm.1, 1);
            } else {
                break;
            }
        }
    }

    line_y = (lines >> 1) * 10 + y;

    if let Some(instruction) = map_lines.get(&cpu.pc) {

        let mut it = map_lines.range_mut((Bound::Unbounded, Bound::Excluded(&cpu.pc)));

        line_y = (lines >> 1) * 10 + y;
        while line_y > y {
            line_y -= 10;

            if let Some(prev_asm) = it.next_back() {
                status.draw(screen, (x as usize, line_y as usize), prev_asm.1, 1);
            } else {
                break;
            }
        }
    }
}


fn main() {
    let mut code_assemble_bin = String::from("A2 0A 8E 00 00 A2 03 8E 01 00 AC 00 00 A9 00 18 6D 01 00 88 D0 FA 8D 02 00 EA EA EA");
    let code_assemble_bin = code_assemble_bin.replace(" ", "");

    let code_bin_result = decode_hex(code_assemble_bin.as_str());

    let code_bin = code_bin_result.expect("failed to get result");

    let mut ram_offset = 0x8000;

    let mut cpu = cpu6502::new();


    for byte_code in code_bin {
        cpu.bus.write(ram_offset, byte_code);
        ram_offset += 1;
    }

    let mut value = 0;

    while value <= 0xFFFF {
        value += 1;
    }


    cpu.bus.write(0xFFFC, 0x00);
    cpu.bus.write(0xFFFD, 0x80);
    let mut map_lines = cpu.disassemble(0x0000, 0xFFFF);

    cpu.reset();


    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new(
        "Test - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
        .unwrap_or_else(|e| {
            panic!("{}", e);
        });

    // Limit to max ~60 fps update rate
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    let status_text = StatusText::new(WIDTH, HEIGHT, 1);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            cpu.reset();
        }

        if window.is_key_pressed(Key::Space, KeyRepeat::No) {
            loop {
                cpu.clock();

                if cpu.complete() {
                    break;
                }
            }
        }


        draw_ram(&status_text, &cpu, &mut buffer, 2, 2, 0x0000, 16, 16);
        draw_ram(&status_text, &cpu, &mut buffer, 2, 182, 0x8000, 16, 16);
        draw_cpu(&status_text, &cpu, &mut buffer, 448, 2);
        draw_code(&status_text, &cpu, &mut buffer, 448, 72, 26, &mut map_lines);


        status_text.draw(&mut buffer, (10, 370), "SPACE = Step Instruction    R = RESET    I = IRQ    N = NMI", 1);

        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        window
            .update_with_buffer(&buffer, WIDTH, HEIGHT)
            .unwrap();
    }


    println!("Hello, world! {:?}", FLAGS6502::N as i8);
}


pub struct StatusText {
    texture: Vec<u32>,
    width: usize,
    //height: usize,
    scale: usize,
}

#[inline(always)]
fn color_from_bit(bit: u8) -> u32 {
    if bit == 0 {
        0x00000000
    } else {
        0xFFFFFFFF
    }
}

impl StatusText {
    pub fn new(width: usize, _height: usize, scale: usize) -> Self {
        // unpack texture for easier drawing
        let mut texture = Vec::with_capacity(128 * 128);

        for t in MICROKNIGHT_FONT {
            texture.push(color_from_bit((t >> 7) & 1));
            texture.push(color_from_bit((t >> 6) & 1));
            texture.push(color_from_bit((t >> 5) & 1));
            texture.push(color_from_bit((t >> 4) & 1));
            texture.push(color_from_bit((t >> 3) & 1));
            texture.push(color_from_bit((t >> 2) & 1));
            texture.push(color_from_bit((t >> 1) & 1));
            texture.push(color_from_bit(t & 1));
        }

        Self {
            texture,
            width,
            //height,
            scale,
        }
    }

    pub fn draw(&self, screen: &mut [u32], pos: (usize, usize), text: &str, color: u32) {
        let mut x = pos.0;
        let y = pos.1;
        for c in text.chars() {
            let mut index = c as usize - ' ' as usize;
            if index > MICROKNIGHT_LAYOUT.len() as usize {
                index = 0;
            }

            let layout = MICROKNIGHT_LAYOUT[index];
            let texture_offset = (layout.1 as usize * 128) + layout.0 as usize;

            for fy in 0..8 * self.scale {
                let ty = fy / self.scale;
                for fx in 0..8 * self.scale {
                    let tx = fx / self.scale;
                    let pixel = texture_offset + (ty * 128) + tx;
                    if pixel != 0 {
                        screen[((y + fy) * self.width) + fx + x] = self.texture[pixel] * color;
                    }
                }
            }

            x += 8 * self.scale;
        }
    }
}


// Microknight font (128x128 packed with 1 bit per pixel)
#[rustfmt::skip]
pub static MICROKNIGHT_FONT: &[u8] = &[
    0x00, 0x0c, 0x1b, 0x0d, 0x81, 0x03, 0x01, 0xc0, 0x30, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x0c, 0x1b, 0x0d, 0x87, 0xc4, 0xb3, 0x60, 0x30, 0x30, 0x0c, 0x1b, 0x03, 0x00, 0x00, 0x00,
    0x00, 0x0c, 0x09, 0x1f, 0xcd, 0x03, 0xe1, 0xc0, 0x10, 0x60, 0x06, 0x0e, 0x03, 0x00, 0x00, 0x00,
    0x00, 0x0c, 0x00, 0x0d, 0x87, 0xc0, 0xc3, 0xd8, 0x20, 0x60, 0x06, 0x3f, 0x8f, 0xc0, 0x03, 0xe0,
    0x00, 0x0c, 0x00, 0x1f, 0xc1, 0x61, 0x83, 0x70, 0x00, 0x60, 0x06, 0x0e, 0x03, 0x01, 0x80, 0x00,
    0x00, 0x00, 0x00, 0x0d, 0x81, 0x63, 0x63, 0x60, 0x00, 0x30, 0x0c, 0x1b, 0x03, 0x01, 0x80, 0x00,
    0x00, 0x0c, 0x00, 0x0d, 0x87, 0xc6, 0x91, 0xf0, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x80, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x60, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x03, 0x07, 0xc1, 0xe0, 0x61, 0xf0, 0x70, 0x7f, 0x1e, 0x0f, 0x00, 0x00, 0x00,
    0x00, 0x03, 0x1e, 0x03, 0x00, 0x60, 0x30, 0x61, 0x80, 0xc0, 0x03, 0x33, 0x19, 0x81, 0x80, 0xc0,
    0x00, 0x06, 0x33, 0x07, 0x03, 0xc0, 0xe0, 0xc1, 0xf8, 0xfc, 0x06, 0x1f, 0x18, 0xc1, 0x80, 0xc0,
    0x00, 0x0c, 0x37, 0x83, 0x06, 0x00, 0x31, 0xb0, 0x0c, 0xc6, 0x0c, 0x31, 0x98, 0xc0, 0x00, 0x00,
    0x00, 0x18, 0x3d, 0x83, 0x0c, 0x02, 0x33, 0x30, 0x8c, 0xc6, 0x0c, 0x31, 0x8f, 0xc0, 0x00, 0x00,
    0x18, 0x30, 0x39, 0x83, 0x0c, 0x06, 0x33, 0xf9, 0x98, 0xcc, 0x0c, 0x33, 0x00, 0xc1, 0x80, 0xc0,
    0x18, 0x60, 0x1f, 0x0f, 0xcf, 0xe3, 0xe0, 0x30, 0xf0, 0x78, 0x0c, 0x1e, 0x03, 0x81, 0x80, 0x40,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x0f, 0x83, 0x83, 0xc3, 0xe0, 0xf0, 0xf8, 0x7f, 0x3f, 0x87, 0x0c, 0x63, 0xf0,
    0x18, 0x00, 0x0c, 0x18, 0xc6, 0xc6, 0x63, 0x31, 0x98, 0xcc, 0x60, 0x30, 0x0c, 0x0c, 0x60, 0xc0,
    0x30, 0x3e, 0x06, 0x00, 0xcd, 0xe6, 0x33, 0xf1, 0x80, 0xc6, 0x7e, 0x3f, 0x18, 0x0c, 0x60, 0xc0,
    0x60, 0x00, 0x03, 0x07, 0x8f, 0x67, 0xf3, 0x19, 0x80, 0xc6, 0x60, 0x30, 0x19, 0xcf, 0xe0, 0xc0,
    0x30, 0x3e, 0x06, 0x06, 0x0d, 0xe6, 0x33, 0x19, 0x80, 0xc6, 0x60, 0x30, 0x18, 0xcc, 0x60, 0xc0,
    0x18, 0x00, 0x0c, 0x00, 0x0c, 0x06, 0x33, 0x31, 0x8c, 0xc6, 0x60, 0x30, 0x18, 0xcc, 0x60, 0xc0,
    0x00, 0x00, 0x00, 0x06, 0x06, 0x66, 0x33, 0xe0, 0xf8, 0xfc, 0x7f, 0x30, 0x0f, 0xcc, 0x63, 0xf0,
    0x00, 0x00, 0x00, 0x00, 0x03, 0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x0e, 0x63, 0x30, 0x18, 0xcc, 0x63, 0xc3, 0xe0, 0xf0, 0xf8, 0x3c, 0x1f, 0x98, 0xcc, 0x66, 0x30,
    0x06, 0x66, 0x30, 0x1d, 0xce, 0x66, 0x63, 0x31, 0x98, 0xcc, 0x60, 0x06, 0x18, 0xcc, 0x66, 0x30,
    0x06, 0x6c, 0x30, 0x1f, 0xcf, 0x66, 0x33, 0x19, 0x8c, 0xc6, 0x3e, 0x06, 0x18, 0xcc, 0x66, 0x30,
    0x06, 0x78, 0x30, 0x1a, 0xcd, 0xe6, 0x33, 0x19, 0x8c, 0xc6, 0x03, 0x06, 0x18, 0xc6, 0xc6, 0xb0,
    0xc6, 0x6c, 0x30, 0x18, 0xcc, 0xe6, 0x33, 0xf1, 0x8c, 0xfc, 0x23, 0x06, 0x18, 0xc6, 0xc7, 0xf0,
    0xc6, 0x66, 0x30, 0x18, 0xcc, 0x66, 0x33, 0x01, 0xac, 0xd8, 0x63, 0x06, 0x18, 0xc3, 0x87, 0x70,
    0x7c, 0x63, 0x3f, 0x98, 0xcc, 0x63, 0xe3, 0x00, 0xf8, 0xcc, 0x3e, 0x06, 0x0f, 0x83, 0x86, 0x30,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xc6, 0x63, 0x3f, 0x87, 0x00, 0x01, 0xc0, 0x40, 0x00, 0x18, 0x00, 0x30, 0x00, 0x00, 0x60, 0x00,
    0x6c, 0x63, 0x03, 0x06, 0x0c, 0x00, 0xc0, 0xe0, 0x00, 0x18, 0x1e, 0x3e, 0x0f, 0x03, 0xe3, 0xc0,
    0x38, 0x63, 0x06, 0x06, 0x06, 0x00, 0xc1, 0xb0, 0x00, 0x10, 0x03, 0x33, 0x19, 0x86, 0x66, 0x60,
    0x38, 0x3e, 0x0c, 0x06, 0x03, 0x00, 0xc0, 0x00, 0x00, 0x08, 0x3f, 0x31, 0x98, 0x0c, 0x67, 0xe0,
    0x6c, 0x06, 0x18, 0x06, 0x01, 0x80, 0xc0, 0x00, 0x00, 0x00, 0x63, 0x31, 0x98, 0x0c, 0x66, 0x00,
    0xc6, 0x06, 0x30, 0x06, 0x00, 0xc0, 0xc0, 0x00, 0x00, 0x00, 0x63, 0x31, 0x98, 0xcc, 0x66, 0x30,
    0xc6, 0x06, 0x3f, 0x87, 0x00, 0x61, 0xc0, 0x00, 0x00, 0x00, 0x3f, 0x3f, 0x0f, 0x87, 0xe3, 0xe0,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xfc, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x38, 0x00, 0x30, 0x03, 0x00, 0xc6, 0x00, 0xe0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x6c, 0x3f, 0x3e, 0x00, 0x00, 0x06, 0x60, 0x61, 0x88, 0xf8, 0x3c, 0x3e, 0x07, 0xcf, 0xc3, 0xc0,
    0x60, 0x63, 0x33, 0x07, 0x01, 0xc6, 0xc0, 0x61, 0xdc, 0xcc, 0x66, 0x33, 0x0c, 0xcc, 0x66, 0x00,
    0x78, 0x63, 0x31, 0x83, 0x00, 0xc7, 0x80, 0x61, 0xfc, 0xc6, 0x63, 0x31, 0x98, 0xcc, 0x03, 0xe0,
    0x60, 0x63, 0x31, 0x83, 0x00, 0xc6, 0xc0, 0x61, 0xac, 0xc6, 0x63, 0x31, 0x98, 0xcc, 0x00, 0x30,
    0x60, 0x3f, 0x31, 0x83, 0x00, 0xc6, 0x60, 0x61, 0x8c, 0xc6, 0x63, 0x31, 0x98, 0xcc, 0x06, 0x30,
    0x60, 0x03, 0x31, 0x8f, 0xc4, 0xc6, 0x31, 0xf9, 0x8c, 0xc6, 0x3e, 0x3f, 0x0f, 0xcc, 0x03, 0xe0,
    0x60, 0x3e, 0x00, 0x00, 0x03, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x00, 0xc0, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x18, 0x1c, 0x87, 0x00, 0x00, 0xc0,
    0x7c, 0x63, 0x31, 0x98, 0xcc, 0x66, 0x33, 0xf8, 0x30, 0x18, 0x0c, 0x27, 0x0e, 0x00, 0x00, 0x00,
    0x30, 0x63, 0x31, 0x9a, 0xc6, 0xc6, 0x30, 0x30, 0x30, 0x18, 0x0c, 0x00, 0x1c, 0x00, 0x00, 0xc0,
    0x30, 0x63, 0x1b, 0x1f, 0xc3, 0x86, 0x30, 0x60, 0x60, 0x18, 0x06, 0x00, 0x18, 0x20, 0x00, 0xc0,
    0x30, 0x63, 0x1b, 0x0f, 0x83, 0x86, 0x30, 0xc0, 0x30, 0x18, 0x0c, 0x00, 0x10, 0x60, 0x00, 0xc0,
    0x32, 0x63, 0x0e, 0x0d, 0x86, 0xc3, 0xf1, 0x80, 0x30, 0x18, 0x0c, 0x00, 0x00, 0xe0, 0x00, 0xc0,
    0x1c, 0x3f, 0x0e, 0x08, 0x8c, 0x60, 0x33, 0xf8, 0x18, 0x18, 0x18, 0x00, 0x01, 0xc0, 0x00, 0xc0,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0xe0, 0x00, 0x00, 0x18, 0x00, 0x00, 0x03, 0x80, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x30, 0x1c, 0x00, 0x18, 0xc1, 0x83, 0xc1, 0xb0, 0x78, 0x00, 0x00, 0x1f, 0x00, 0x03, 0xc3, 0xe0,
    0x78, 0x36, 0x31, 0x98, 0xc1, 0x86, 0x00, 0x00, 0x84, 0x7e, 0x1b, 0x03, 0x00, 0x04, 0x20, 0x00,
    0xcc, 0x30, 0x1f, 0x18, 0xc1, 0x83, 0xe0, 0x01, 0x32, 0xc6, 0x36, 0x00, 0x00, 0x0b, 0x90, 0x00,
    0xc0, 0x7c, 0x31, 0x8f, 0x80, 0x06, 0x30, 0x01, 0x42, 0xc6, 0x6c, 0x00, 0x0f, 0x8a, 0x50, 0x00,
    0xc0, 0x30, 0x31, 0x81, 0x81, 0x86, 0x30, 0x01, 0x42, 0x7e, 0x36, 0x00, 0x00, 0x0b, 0x90, 0x00,
    0xc6, 0x30, 0x1f, 0x07, 0xc1, 0x83, 0xe0, 0x01, 0x32, 0x00, 0x1b, 0x00, 0x00, 0x0a, 0x50, 0x00,
    0x7c, 0x7f, 0x31, 0x81, 0x81, 0x80, 0x30, 0x00, 0x84, 0x7c, 0x00, 0x00, 0x00, 0x04, 0x20, 0x00,
    0x30, 0x00, 0x00, 0x00, 0x00, 0x01, 0xe0, 0x00, 0x78, 0x00, 0x00, 0x00, 0x00, 0x03, 0xc0, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x38, 0x00, 0x1c, 0x0e, 0x01, 0x80, 0x00, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x06, 0x03, 0x00,
    0x6c, 0x08, 0x06, 0x03, 0x03, 0x06, 0x31, 0xf8, 0x00, 0x00, 0x38, 0x1f, 0x1b, 0x0e, 0x67, 0x30,
    0x6c, 0x3e, 0x0c, 0x06, 0x06, 0x06, 0x33, 0xd0, 0x30, 0x00, 0x18, 0x31, 0x8d, 0x86, 0xc3, 0x60,
    0x38, 0x08, 0x18, 0x03, 0x00, 0x06, 0x31, 0xd0, 0x30, 0x00, 0x18, 0x31, 0x86, 0xc7, 0xa3, 0xc0,
    0x00, 0x00, 0x1e, 0x0e, 0x00, 0x06, 0x30, 0x50, 0x00, 0x00, 0x3c, 0x1f, 0x0d, 0x83, 0x61, 0xf0,
    0x00, 0x3e, 0x00, 0x00, 0x00, 0x06, 0x30, 0x50, 0x00, 0x00, 0x00, 0x00, 0x1b, 0x06, 0xf3, 0x18,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0xe0, 0x50, 0x00, 0x18, 0x00, 0x1f, 0x00, 0x0c, 0xf6, 0x70,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x30, 0x00, 0x00, 0x00, 0x00, 0x30, 0xf8,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xe0, 0x18, 0x0c, 0x03, 0x03, 0x03, 0x91, 0xb0, 0xf0, 0x3f, 0x3c, 0x0c, 0x03, 0x01, 0x83, 0x60,
    0x36, 0x00, 0x02, 0x04, 0x0c, 0xc4, 0xe0, 0x01, 0x98, 0x6c, 0x66, 0x02, 0x04, 0x06, 0x60, 0x00,
    0x6c, 0x18, 0x1e, 0x0f, 0x07, 0x83, 0xc1, 0xe0, 0xf0, 0xcf, 0x60, 0x3f, 0x9f, 0xcf, 0xe7, 0xf0,
    0x3a, 0x1e, 0x33, 0x19, 0x8c, 0xc6, 0x63, 0x31, 0x98, 0xfc, 0x60, 0x30, 0x18, 0x0c, 0x06, 0x00,
    0xf6, 0x03, 0x3f, 0x9f, 0xcf, 0xe7, 0xf3, 0xf9, 0xfc, 0xcc, 0x60, 0x3f, 0x1f, 0x8f, 0xc7, 0xe0,
    0x6f, 0x63, 0x31, 0x98, 0xcc, 0x66, 0x33, 0x19, 0x8c, 0xcc, 0x63, 0x30, 0x18, 0x0c, 0x06, 0x00,
    0xcf, 0x3e, 0x31, 0x98, 0xcc, 0x66, 0x33, 0x19, 0x8c, 0xcf, 0x3e, 0x3f, 0x9f, 0xcf, 0xe7, 0xf0,
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x30, 0x0c, 0x06, 0x0d, 0x8f, 0x83, 0x90, 0xc0, 0x30, 0x30, 0x39, 0x1b, 0x00, 0x07, 0x81, 0x80,
    0x08, 0x10, 0x19, 0x80, 0x0c, 0xc4, 0xe0, 0x20, 0x40, 0xcc, 0x4e, 0x00, 0x0f, 0x8c, 0xc0, 0x40,
    0x7e, 0x3f, 0x1f, 0x8f, 0xcc, 0x67, 0x31, 0xe0, 0xf0, 0x78, 0x3c, 0x1e, 0x1a, 0xcd, 0xe6, 0x30,
    0x18, 0x0c, 0x06, 0x03, 0x0e, 0x67, 0xb3, 0x31, 0x98, 0xcc, 0x66, 0x33, 0x1f, 0xef, 0x66, 0x30,
    0x18, 0x0c, 0x06, 0x03, 0x0c, 0x66, 0xf3, 0x19, 0x8c, 0xc6, 0x63, 0x31, 0x9b, 0x6e, 0x66, 0x30,
    0x18, 0x0c, 0x06, 0x03, 0x0c, 0x66, 0x73, 0x19, 0x8c, 0xc6, 0x63, 0x31, 0x98, 0xec, 0x66, 0x30,
    0x7e, 0x3f, 0x1f, 0x8f, 0xcf, 0xc6, 0x31, 0xf0, 0xf8, 0x7c, 0x3e, 0x1f, 0x0f, 0xc7, 0xc3, 0xe0,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x18, 0x0c, 0x1b, 0x03, 0x0c, 0x00, 0x00, 0xc0, 0x30, 0x18, 0x39, 0x1b, 0x07, 0x80, 0x00, 0x00,
    0x20, 0x33, 0x00, 0x04, 0x0f, 0x83, 0xc0, 0x20, 0x40, 0x66, 0x4e, 0x00, 0x0c, 0xc7, 0xe3, 0xc0,
    0xc6, 0x63, 0x31, 0x98, 0xcc, 0xc6, 0x60, 0xf0, 0x78, 0x3c, 0x1e, 0x0f, 0x07, 0x81, 0xb6, 0x60,
    0xc6, 0x63, 0x31, 0x98, 0xcc, 0x66, 0xe0, 0x18, 0x0c, 0x06, 0x03, 0x01, 0x80, 0xc7, 0xf6, 0x00,
    0xc6, 0x63, 0x31, 0x8f, 0x8f, 0xc6, 0x31, 0xf8, 0xfc, 0x7e, 0x3f, 0x1f, 0x8f, 0xcd, 0x86, 0x00,
    0xc6, 0x63, 0x31, 0x81, 0x8c, 0x06, 0x33, 0x19, 0x8c, 0xc6, 0x63, 0x31, 0x98, 0xcd, 0x86, 0x30,
    0x7c, 0x3e, 0x1f, 0x01, 0x8c, 0x06, 0xe1, 0xf8, 0xfc, 0x7e, 0x3f, 0x1f, 0x8f, 0xc7, 0xf3, 0xe0,
    0x00, 0x00, 0x00, 0x00, 0x0c, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x80,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x30, 0x0c, 0x0c, 0x0d, 0x83, 0x00, 0xc0, 0x60, 0xd8, 0x0c, 0x39, 0x0c, 0x03, 0x01, 0x83, 0x90,
    0x08, 0x10, 0x33, 0x00, 0x00, 0x81, 0x01, 0x98, 0x00, 0x16, 0x4e, 0x02, 0x04, 0x06, 0x64, 0xe0,
    0x78, 0x3c, 0x1e, 0x0f, 0x03, 0x81, 0xc0, 0xe0, 0x70, 0x3e, 0x7c, 0x1e, 0x0f, 0x07, 0x83, 0xc0,
    0xfc, 0x7e, 0x3f, 0x1f, 0x81, 0x80, 0xc0, 0x60, 0x30, 0x66, 0x66, 0x33, 0x19, 0x8c, 0xc6, 0x60,
    0xc0, 0x60, 0x30, 0x18, 0x01, 0x80, 0xc0, 0x60, 0x30, 0xc6, 0x63, 0x31, 0x98, 0xcc, 0x66, 0x30,
    0xc6, 0x63, 0x31, 0x98, 0xc1, 0x80, 0xc0, 0x60, 0x30, 0xc6, 0x63, 0x31, 0x98, 0xcc, 0x66, 0x30,
    0x7c, 0x3e, 0x1f, 0x0f, 0x87, 0xe3, 0xf1, 0xf8, 0xfc, 0x7e, 0x63, 0x1f, 0x0f, 0x87, 0xc3, 0xe0,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x6c, 0x00, 0x00, 0x06, 0x01, 0x80, 0xc1, 0xb0, 0x30, 0xc0, 0x36, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x0c, 0x1e, 0x01, 0x02, 0x03, 0x30, 0x00, 0x40, 0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x78, 0x00, 0x33, 0x18, 0xcc, 0x66, 0x33, 0x19, 0x8c, 0xf8, 0x63, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xcc, 0x3f, 0x37, 0x98, 0xcc, 0x66, 0x33, 0x19, 0x8c, 0xcc, 0x63, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xc6, 0x00, 0x3d, 0x98, 0xcc, 0x66, 0x33, 0x19, 0x8c, 0xc6, 0x63, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xc6, 0x0c, 0x39, 0x98, 0xcc, 0x66, 0x33, 0x18, 0xfc, 0xfc, 0x3f, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x7c, 0x00, 0x1f, 0x0f, 0xc7, 0xe3, 0xf1, 0xf8, 0x0c, 0xc0, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf8, 0xc0, 0x3e, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// Font layout (generated from Angelcode Bitmap Font generator)
#[rustfmt::skip]
pub static MICROKNIGHT_LAYOUT: &[(u8, u8)] = &[
    (0, 0), (9, 0), (18, 0), (27, 0), (36, 0), (45, 0), (54, 0), (63, 0), (72, 0), (81, 0), (90, 0), (99, 0), (108, 0),
    (117, 0), (0, 9), (9, 9), (18, 9), (27, 9), (36, 9), (45, 9), (54, 9), (63, 9), (72, 9), (81, 9), (90, 9), (99, 9),
    (108, 9), (117, 9), (0, 18), (9, 18), (18, 18), (27, 18), (36, 18), (45, 18), (54, 18), (63, 18), (72, 18), (81, 18),
    (90, 18), (99, 18), (108, 18), (117, 18), (0, 27), (9, 27), (18, 27), (27, 27), (36, 27), (45, 27), (54, 27), (63, 27),
    (72, 27), (81, 27), (90, 27), (99, 27), (108, 27), (117, 27), (0, 36), (9, 36), (18, 36), (27, 36), (36, 36), (45, 36),
    (54, 36), (63, 36), (72, 36), (81, 36), (90, 36), (99, 36), (108, 36), (117, 36), (0, 45), (9, 45), (18, 45), (27, 45),
    (36, 45), (45, 45), (54, 45), (63, 45), (72, 45), (81, 45), (90, 45), (99, 45), (108, 45), (117, 45), (0, 54), (9, 54),
    (18, 54), (27, 54), (36, 54), (45, 54), (54, 54), (63, 54), (72, 54), (81, 54), (90, 54), (99, 54), (0, 0), (0, 0),
    (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0),
    (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0),
    (108, 54), (117, 54), (0, 63), (9, 63), (18, 63), (27, 63), (36, 63), (45, 63), (54, 63), (63, 63), (72, 63), (81, 63),
    (90, 63), (99, 63), (108, 63), (117, 63), (0, 72), (9, 72), (18, 72), (27, 72), (36, 72), (45, 72), (54, 72), (63, 72),
    (72, 72), (81, 72), (90, 72), (99, 72), (108, 72), (117, 72), (0, 81), (9, 81), (18, 81), (27, 81), (36, 81), (45, 81),
    (54, 81), (63, 81), (72, 81), (81, 81), (90, 81), (99, 81), (108, 81), (117, 81), (0, 90), (9, 90), (18, 90), (27, 90),
    (36, 90), (45, 90), (54, 90), (63, 90), (72, 90), (81, 90), (90, 90), (99, 90), (108, 90), (117, 90), (0, 99), (9, 99),
    (18, 99), (27, 99), (36, 99), (45, 99), (54, 99), (63, 99), (72, 99), (81, 99), (90, 99), (99, 99), (108, 99), (117, 99),
    (0, 108), (9, 108), (18, 108), (27, 108), (36, 108), (45, 108), (54, 108), (63, 108), (72, 108), (81, 108), (90, 108),
    (99, 108), (108, 108), (117, 108), (0, 117), (9, 117), (18, 117), (27, 117), (36, 117), (45, 117), (54, 117), (63, 117),
    (72, 117), (81, 117),
];
