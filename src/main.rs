use std::fs::read;
use std::rc::Rc;

type RamArray = [u8; 64 * 1024];

struct Bus
{
    ram: RamArray,
    cpu: cpu6502,
}

impl Bus
{
    fn new() -> Self {
        let cpu = cpu6502::new();


        return Bus {
            ram: [0; 64 * 1024],
            cpu,
        };
    }

    fn write(&mut self, addr: u16, data: u8)
    {
        if addr >= 0x0000 && addr <= 0xFFFF {
            self.ram[addr as usize] = data;
        }
    }

    fn read(&self, addr: u16, read_only: bool) -> u8
    {
        if addr >= 0x0000 && addr <= 0xFFFF {
            // let v = self.ram.get(addr).expect("Failed to read value from array").collect();
            return self.ram[addr as usize];
        }

        return 0x00;
    }
}

#[derive(Debug)]
#[repr(i8)]
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
    bus: Rc<Bus>,
    clock_count: u32,
    temp: u16,
}

type cpu = cpu6502;

impl cpu6502 {
    fn new() -> Self {
        let lookup: Vec<INSTRUCTION> = vec![
            INSTRUCTION { name: "BRK".to_string(), operate: cpu::BRK, addr_mode: cpu::IMM, cycles: 7 }, INSTRUCTION { name: "ORA".to_string(), operate: cpu::ORA, addr_mode: cpu::IZX, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 8 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 3 }, INSTRUCTION { name: "ORA".to_string(), operate: cpu::ORA, addr_mode: cpu::ZP0, cycles: 3 }, INSTRUCTION { name: "ASL".to_string(), operate: cpu::ASL, addr_mode: cpu::ZP0, cycles: 5 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 5 }, INSTRUCTION { name: "PHP".to_string(), operate: cpu::PHP, addr_mode: cpu::IMP, cycles: 3 }, INSTRUCTION { name: "ORA".to_string(), operate: cpu::ORA, addr_mode: cpu::IMM, cycles: 2 }, INSTRUCTION { name: "ASL".to_string(), operate: cpu::ASL, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 4 }, INSTRUCTION { name: "ORA".to_string(), operate: cpu::ORA, addr_mode: cpu::ABS, cycles: 4 }, INSTRUCTION { name: "ASL".to_string(), operate: cpu::ASL, addr_mode: cpu::ABS, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 6 },
            INSTRUCTION { name: "BPL".to_string(), operate: cpu::BPL, addr_mode: cpu::REL, cycles: 2 }, INSTRUCTION { name: "ORA".to_string(), operate: cpu::ORA, addr_mode: cpu::IZY, cycles: 5 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 8 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 4 }, INSTRUCTION { name: "ORA".to_string(), operate: cpu::ORA, addr_mode: cpu::ZPX, cycles: 4 }, INSTRUCTION { name: "ASL".to_string(), operate: cpu::ASL, addr_mode: cpu::ZPX, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 6 }, INSTRUCTION { name: "CLC".to_string(), operate: cpu::CLC, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "ORA".to_string(), operate: cpu::ORA, addr_mode: cpu::ABY, cycles: 4 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 7 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 4 }, INSTRUCTION { name: "ORA".to_string(), operate: cpu::ORA, addr_mode: cpu::ABX, cycles: 4 }, INSTRUCTION { name: "ASL".to_string(), operate: cpu::ASL, addr_mode: cpu::ABX, cycles: 7 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 7 },
            INSTRUCTION { name: "JSR".to_string(), operate: cpu::JSR, addr_mode: cpu::ABS, cycles: 6 }, INSTRUCTION { name: "AND".to_string(), operate: cpu::AND, addr_mode: cpu::IZX, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 8 }, INSTRUCTION { name: "BIT".to_string(), operate: cpu::BIT, addr_mode: cpu::ZP0, cycles: 3 }, INSTRUCTION { name: "AND".to_string(), operate: cpu::AND, addr_mode: cpu::ZP0, cycles: 3 }, INSTRUCTION { name: "ROL".to_string(), operate: cpu::ROL, addr_mode: cpu::ZP0, cycles: 5 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 5 }, INSTRUCTION { name: "PLP".to_string(), operate: cpu::PLP, addr_mode: cpu::IMP, cycles: 4 }, INSTRUCTION { name: "AND".to_string(), operate: cpu::AND, addr_mode: cpu::IMM, cycles: 2 }, INSTRUCTION { name: "ROL".to_string(), operate: cpu::ROL, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "BIT".to_string(), operate: cpu::BIT, addr_mode: cpu::ABS, cycles: 4 }, INSTRUCTION { name: "AND".to_string(), operate: cpu::AND, addr_mode: cpu::ABS, cycles: 4 }, INSTRUCTION { name: "ROL".to_string(), operate: cpu::ROL, addr_mode: cpu::ABS, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 6 },
            INSTRUCTION { name: "BMI".to_string(), operate: cpu::BMI, addr_mode: cpu::REL, cycles: 2 }, INSTRUCTION { name: "AND".to_string(), operate: cpu::AND, addr_mode: cpu::IZY, cycles: 5 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 8 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 4 }, INSTRUCTION { name: "AND".to_string(), operate: cpu::AND, addr_mode: cpu::ZPX, cycles: 4 }, INSTRUCTION { name: "ROL".to_string(), operate: cpu::ROL, addr_mode: cpu::ZPX, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 6 }, INSTRUCTION { name: "SEC".to_string(), operate: cpu::SEC, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "AND".to_string(), operate: cpu::AND, addr_mode: cpu::ABY, cycles: 4 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 7 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 4 }, INSTRUCTION { name: "AND".to_string(), operate: cpu::AND, addr_mode: cpu::ABX, cycles: 4 }, INSTRUCTION { name: "ROL".to_string(), operate: cpu::ROL, addr_mode: cpu::ABX, cycles: 7 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 7 },
            INSTRUCTION { name: "RTI".to_string(), operate: cpu::RTI, addr_mode: cpu::IMP, cycles: 6 }, INSTRUCTION { name: "EOR".to_string(), operate: cpu::EOR, addr_mode: cpu::IZX, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 8 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 3 }, INSTRUCTION { name: "EOR".to_string(), operate: cpu::EOR, addr_mode: cpu::ZP0, cycles: 3 }, INSTRUCTION { name: "LSR".to_string(), operate: cpu::LSR, addr_mode: cpu::ZP0, cycles: 5 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 5 }, INSTRUCTION { name: "PHA".to_string(), operate: cpu::PHA, addr_mode: cpu::IMP, cycles: 3 }, INSTRUCTION { name: "EOR".to_string(), operate: cpu::EOR, addr_mode: cpu::IMM, cycles: 2 }, INSTRUCTION { name: "LSR".to_string(), operate: cpu::LSR, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "JMP".to_string(), operate: cpu::JMP, addr_mode: cpu::ABS, cycles: 3 }, INSTRUCTION { name: "EOR".to_string(), operate: cpu::EOR, addr_mode: cpu::ABS, cycles: 4 }, INSTRUCTION { name: "LSR".to_string(), operate: cpu::LSR, addr_mode: cpu::ABS, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 6 },
            INSTRUCTION { name: "BVC".to_string(), operate: cpu::BVC, addr_mode: cpu::REL, cycles: 2 }, INSTRUCTION { name: "EOR".to_string(), operate: cpu::EOR, addr_mode: cpu::IZY, cycles: 5 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 8 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 4 }, INSTRUCTION { name: "EOR".to_string(), operate: cpu::EOR, addr_mode: cpu::ZPX, cycles: 4 }, INSTRUCTION { name: "LSR".to_string(), operate: cpu::LSR, addr_mode: cpu::ZPX, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 6 }, INSTRUCTION { name: "CLI".to_string(), operate: cpu::CLI, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "EOR".to_string(), operate: cpu::EOR, addr_mode: cpu::ABY, cycles: 4 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 7 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 4 }, INSTRUCTION { name: "EOR".to_string(), operate: cpu::EOR, addr_mode: cpu::ABX, cycles: 4 }, INSTRUCTION { name: "LSR".to_string(), operate: cpu::LSR, addr_mode: cpu::ABX, cycles: 7 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 7 },
            INSTRUCTION { name: "RTS".to_string(), operate: cpu::RTS, addr_mode: cpu::IMP, cycles: 6 }, INSTRUCTION { name: "ADC".to_string(), operate: cpu::ADC, addr_mode: cpu::IZX, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 8 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 3 }, INSTRUCTION { name: "ADC".to_string(), operate: cpu::ADC, addr_mode: cpu::ZP0, cycles: 3 }, INSTRUCTION { name: "ROR".to_string(), operate: cpu::ROR, addr_mode: cpu::ZP0, cycles: 5 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 5 }, INSTRUCTION { name: "PLA".to_string(), operate: cpu::PLA, addr_mode: cpu::IMP, cycles: 4 }, INSTRUCTION { name: "ADC".to_string(), operate: cpu::ADC, addr_mode: cpu::IMM, cycles: 2 }, INSTRUCTION { name: "ROR".to_string(), operate: cpu::ROR, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "JMP".to_string(), operate: cpu::JMP, addr_mode: cpu::IND, cycles: 5 }, INSTRUCTION { name: "ADC".to_string(), operate: cpu::ADC, addr_mode: cpu::ABS, cycles: 4 }, INSTRUCTION { name: "ROR".to_string(), operate: cpu::ROR, addr_mode: cpu::ABS, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 6 },
            INSTRUCTION { name: "BVS".to_string(), operate: cpu::BVS, addr_mode: cpu::REL, cycles: 2 }, INSTRUCTION { name: "ADC".to_string(), operate: cpu::ADC, addr_mode: cpu::IZY, cycles: 5 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 8 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 4 }, INSTRUCTION { name: "ADC".to_string(), operate: cpu::ADC, addr_mode: cpu::ZPX, cycles: 4 }, INSTRUCTION { name: "ROR".to_string(), operate: cpu::ROR, addr_mode: cpu::ZPX, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 6 }, INSTRUCTION { name: "SEI".to_string(), operate: cpu::SEI, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "ADC".to_string(), operate: cpu::ADC, addr_mode: cpu::ABY, cycles: 4 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 7 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 4 }, INSTRUCTION { name: "ADC".to_string(), operate: cpu::ADC, addr_mode: cpu::ABX, cycles: 4 }, INSTRUCTION { name: "ROR".to_string(), operate: cpu::ROR, addr_mode: cpu::ABX, cycles: 7 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 7 },
            INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "STA".to_string(), operate: cpu::STA, addr_mode: cpu::IZX, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 6 }, INSTRUCTION { name: "STY".to_string(), operate: cpu::STY, addr_mode: cpu::ZP0, cycles: 3 }, INSTRUCTION { name: "STA".to_string(), operate: cpu::STA, addr_mode: cpu::ZP0, cycles: 3 }, INSTRUCTION { name: "STX".to_string(), operate: cpu::STX, addr_mode: cpu::ZP0, cycles: 3 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 3 }, INSTRUCTION { name: "DEY".to_string(), operate: cpu::DEY, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "TXA".to_string(), operate: cpu::TXA, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "STY".to_string(), operate: cpu::STY, addr_mode: cpu::ABS, cycles: 4 }, INSTRUCTION { name: "STA".to_string(), operate: cpu::STA, addr_mode: cpu::ABS, cycles: 4 }, INSTRUCTION { name: "STX".to_string(), operate: cpu::STX, addr_mode: cpu::ABS, cycles: 4 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 4 },
            INSTRUCTION { name: "BCC".to_string(), operate: cpu::BCC, addr_mode: cpu::REL, cycles: 2 }, INSTRUCTION { name: "STA".to_string(), operate: cpu::STA, addr_mode: cpu::IZY, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 6 }, INSTRUCTION { name: "STY".to_string(), operate: cpu::STY, addr_mode: cpu::ZPX, cycles: 4 }, INSTRUCTION { name: "STA".to_string(), operate: cpu::STA, addr_mode: cpu::ZPX, cycles: 4 }, INSTRUCTION { name: "STX".to_string(), operate: cpu::STX, addr_mode: cpu::ZPY, cycles: 4 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 4 }, INSTRUCTION { name: "TYA".to_string(), operate: cpu::TYA, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "STA".to_string(), operate: cpu::STA, addr_mode: cpu::ABY, cycles: 5 }, INSTRUCTION { name: "TXS".to_string(), operate: cpu::TXS, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 5 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 5 }, INSTRUCTION { name: "STA".to_string(), operate: cpu::STA, addr_mode: cpu::ABX, cycles: 5 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 5 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 5 },
            INSTRUCTION { name: "LDY".to_string(), operate: cpu::LDY, addr_mode: cpu::IMM, cycles: 2 }, INSTRUCTION { name: "LDA".to_string(), operate: cpu::LDA, addr_mode: cpu::IZX, cycles: 6 }, INSTRUCTION { name: "LDX".to_string(), operate: cpu::LDX, addr_mode: cpu::IMM, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 6 }, INSTRUCTION { name: "LDY".to_string(), operate: cpu::LDY, addr_mode: cpu::ZP0, cycles: 3 }, INSTRUCTION { name: "LDA".to_string(), operate: cpu::LDA, addr_mode: cpu::ZP0, cycles: 3 }, INSTRUCTION { name: "LDX".to_string(), operate: cpu::LDX, addr_mode: cpu::ZP0, cycles: 3 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 3 }, INSTRUCTION { name: "TAY".to_string(), operate: cpu::TAY, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "LDA".to_string(), operate: cpu::LDA, addr_mode: cpu::IMM, cycles: 2 }, INSTRUCTION { name: "TAX".to_string(), operate: cpu::TAX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "LDY".to_string(), operate: cpu::LDY, addr_mode: cpu::ABS, cycles: 4 }, INSTRUCTION { name: "LDA".to_string(), operate: cpu::LDA, addr_mode: cpu::ABS, cycles: 4 }, INSTRUCTION { name: "LDX".to_string(), operate: cpu::LDX, addr_mode: cpu::ABS, cycles: 4 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 4 },
            INSTRUCTION { name: "BCS".to_string(), operate: cpu::BCS, addr_mode: cpu::REL, cycles: 2 }, INSTRUCTION { name: "LDA".to_string(), operate: cpu::LDA, addr_mode: cpu::IZY, cycles: 5 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 5 }, INSTRUCTION { name: "LDY".to_string(), operate: cpu::LDY, addr_mode: cpu::ZPX, cycles: 4 }, INSTRUCTION { name: "LDA".to_string(), operate: cpu::LDA, addr_mode: cpu::ZPX, cycles: 4 }, INSTRUCTION { name: "LDX".to_string(), operate: cpu::LDX, addr_mode: cpu::ZPY, cycles: 4 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 4 }, INSTRUCTION { name: "CLV".to_string(), operate: cpu::CLV, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "LDA".to_string(), operate: cpu::LDA, addr_mode: cpu::ABY, cycles: 4 }, INSTRUCTION { name: "TSX".to_string(), operate: cpu::TSX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 4 }, INSTRUCTION { name: "LDY".to_string(), operate: cpu::LDY, addr_mode: cpu::ABX, cycles: 4 }, INSTRUCTION { name: "LDA".to_string(), operate: cpu::LDA, addr_mode: cpu::ABX, cycles: 4 }, INSTRUCTION { name: "LDX".to_string(), operate: cpu::LDX, addr_mode: cpu::ABY, cycles: 4 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 4 },
            INSTRUCTION { name: "CPY".to_string(), operate: cpu::CPY, addr_mode: cpu::IMM, cycles: 2 }, INSTRUCTION { name: "CMP".to_string(), operate: cpu::CMP, addr_mode: cpu::IZX, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 8 }, INSTRUCTION { name: "CPY".to_string(), operate: cpu::CPY, addr_mode: cpu::ZP0, cycles: 3 }, INSTRUCTION { name: "CMP".to_string(), operate: cpu::CMP, addr_mode: cpu::ZP0, cycles: 3 }, INSTRUCTION { name: "DEC".to_string(), operate: cpu::DEC, addr_mode: cpu::ZP0, cycles: 5 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 5 }, INSTRUCTION { name: "INY".to_string(), operate: cpu::INY, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "CMP".to_string(), operate: cpu::CMP, addr_mode: cpu::IMM, cycles: 2 }, INSTRUCTION { name: "DEX".to_string(), operate: cpu::DEX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "CPY".to_string(), operate: cpu::CPY, addr_mode: cpu::ABS, cycles: 4 }, INSTRUCTION { name: "CMP".to_string(), operate: cpu::CMP, addr_mode: cpu::ABS, cycles: 4 }, INSTRUCTION { name: "DEC".to_string(), operate: cpu::DEC, addr_mode: cpu::ABS, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 6 },
            INSTRUCTION { name: "BNE".to_string(), operate: cpu::BNE, addr_mode: cpu::REL, cycles: 2 }, INSTRUCTION { name: "CMP".to_string(), operate: cpu::CMP, addr_mode: cpu::IZY, cycles: 5 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 8 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 4 }, INSTRUCTION { name: "CMP".to_string(), operate: cpu::CMP, addr_mode: cpu::ZPX, cycles: 4 }, INSTRUCTION { name: "DEC".to_string(), operate: cpu::DEC, addr_mode: cpu::ZPX, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 6 }, INSTRUCTION { name: "CLD".to_string(), operate: cpu::CLD, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "CMP".to_string(), operate: cpu::CMP, addr_mode: cpu::ABY, cycles: 4 }, INSTRUCTION { name: "NOP".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 7 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 4 }, INSTRUCTION { name: "CMP".to_string(), operate: cpu::CMP, addr_mode: cpu::ABX, cycles: 4 }, INSTRUCTION { name: "DEC".to_string(), operate: cpu::DEC, addr_mode: cpu::ABX, cycles: 7 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 7 },
            INSTRUCTION { name: "CPX".to_string(), operate: cpu::CPX, addr_mode: cpu::IMM, cycles: 2 }, INSTRUCTION { name: "SBC".to_string(), operate: cpu::SBC, addr_mode: cpu::IZX, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 8 }, INSTRUCTION { name: "CPX".to_string(), operate: cpu::CPX, addr_mode: cpu::ZP0, cycles: 3 }, INSTRUCTION { name: "SBC".to_string(), operate: cpu::SBC, addr_mode: cpu::ZP0, cycles: 3 }, INSTRUCTION { name: "INC".to_string(), operate: cpu::INC, addr_mode: cpu::ZP0, cycles: 5 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 5 }, INSTRUCTION { name: "INX".to_string(), operate: cpu::INX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "SBC".to_string(), operate: cpu::SBC, addr_mode: cpu::IMM, cycles: 2 }, INSTRUCTION { name: "NOP".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::SBC, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "CPX".to_string(), operate: cpu::CPX, addr_mode: cpu::ABS, cycles: 4 }, INSTRUCTION { name: "SBC".to_string(), operate: cpu::SBC, addr_mode: cpu::ABS, cycles: 4 }, INSTRUCTION { name: "INC".to_string(), operate: cpu::INC, addr_mode: cpu::ABS, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 6 },
            INSTRUCTION { name: "BEQ".to_string(), operate: cpu::BEQ, addr_mode: cpu::REL, cycles: 2 }, INSTRUCTION { name: "SBC".to_string(), operate: cpu::SBC, addr_mode: cpu::IZY, cycles: 5 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 8 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 4 }, INSTRUCTION { name: "SBC".to_string(), operate: cpu::SBC, addr_mode: cpu::ZPX, cycles: 4 }, INSTRUCTION { name: "INC".to_string(), operate: cpu::INC, addr_mode: cpu::ZPX, cycles: 6 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 6 }, INSTRUCTION { name: "SED".to_string(), operate: cpu::SED, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "SBC".to_string(), operate: cpu::SBC, addr_mode: cpu::ABY, cycles: 4 }, INSTRUCTION { name: "NOP".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 2 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 7 }, INSTRUCTION { name: "???".to_string(), operate: cpu::NOP, addr_mode: cpu::IMP, cycles: 4 }, INSTRUCTION { name: "SBC".to_string(), operate: cpu::SBC, addr_mode: cpu::ABX, cycles: 4 }, INSTRUCTION { name: "INC".to_string(), operate: cpu::INC, addr_mode: cpu::ABX, cycles: 7 }, INSTRUCTION { name: "???".to_string(), operate: cpu::XXX, addr_mode: cpu::IMP, cycles: 7 },
        ];

        return cpu6502 {
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
            lookup: vec![],
            bus: Rc::new(Bus::new()),
            clock_count: 0,
            temp: 0,
        };
    }

    fn get_flag(&self, f: FLAGS6502) -> u8 {
        let f = f as u8;
        if (self.status & f) > 0 { 1 } else { 0 }
    }

    fn set_flag(&mut self, f: FLAGS6502, v: bool)
    {
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

        return 0
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

        let lo = cpu.read(cpu.pc);
        cpu.pc += 1;
        let hi = cpu.read(cpu.pc);
        cpu.pc += 1;

        cpu.addr_abs = ((hi << 8) | lo) as u16;

        0
    }
    fn ABX(cpu: &mut cpu6502) -> u8 {

        let lo = cpu.read(cpu.pc);
        cpu.pc += 1;
        let hi = cpu.read(cpu.pc);
        cpu.pc += 1;

        cpu.addr_abs = ((hi << 8) | lo) as u16;
        cpu.addr_abs += cpu.x as u16;

        if (cpu.addr_abs & 0xFF00) != (hi << 8) as u16 { 1 } else { 0 }

    }


    fn ABY(cpu: &mut cpu6502) -> u8 {
        let lo = cpu.read(cpu.pc);
        cpu.pc += 1;
        let hi = cpu.read(cpu.pc);
        cpu.pc += 1;

        cpu.addr_abs = ((hi << 8) | lo) as u16;
        cpu.addr_abs += cpu.y as u16;

        if (cpu.addr_abs & 0xFF00) != (hi << 8) as u16 { 1 } else { 0 }

    }

    fn IND(cpu: &mut cpu6502) -> u8 {

        let ptr_lo = cpu.read(cpu.pc);
        cpu.pc += 1;
        let ptr_hi = cpu.read(cpu.pc);
        cpu.pc += 1;

        let ptr = (ptr_hi << 8) | ptr_lo;

        if ptr_lo == 0x00FF // Simulate page boundary hardware bug
        {
            cpu.addr_abs = ((cpu.read((ptr & 0xFFu8) as u16) << 8) | cpu.read((ptr + 0) as u16)) as u16;
        }
        else // Behave normally
        {
            cpu.addr_abs = ((cpu.read((ptr + 1) as u16) << 8) | cpu.read((ptr + 0) as u16)) as u16;
        }

        0
    }

    fn IZX(cpu: &mut cpu6502) -> u8 {

        let t = cpu.read(cpu.pc);
        cpu.pc += 1;

        let lo = cpu.read(((t + cpu.x) & 0x00FF) as u16);
        let hi = cpu.read(((t + cpu.x + 1) & 0x00FF) as u16);

        cpu.addr_abs = ((hi << 8) | lo) as u16;

        0
    }
    fn IZY(cpu: &mut cpu6502) -> u8 {

        let t = cpu.read(cpu.pc);
        cpu.pc += 1;

        let lo = cpu.read((t & 0x00FF) as u16);
        let hi = cpu.read(((t + 1) & 0x00FF) as u16);

        cpu.addr_abs = ((hi << 8) | lo) as u16;
        cpu.addr_abs += cpu.y as u16;

        if (cpu.addr_abs & 0xFF00) != (hi << 8) as u16 { 1 } else { 0 }
    }

    //opcodes
    fn ADC(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn AND(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn ASL(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn BCC(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn BCS(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn BEQ(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn BIT(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn BMI(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn BNE(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn BPL(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn BRK(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn BVC(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn BVS(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn CLC(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn CLD(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn CLI(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn CLV(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn CMP(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn CPX(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn CPY(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn DEC(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn DEX(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn DEY(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn EOR(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn INC(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn INX(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn INY(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn JMP(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn JSR(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn LDA(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn LDX(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn LDY(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn LSR(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn NOP(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn ORA(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn PHA(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn PHP(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn PLA(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn PLP(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn ROL(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn ROR(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn RTI(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn RTS(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn SBC(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn SEC(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn SED(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn SEI(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn STA(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn STX(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn STY(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn TAX(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn TAY(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn TSX(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn TXA(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn TXS(cpu: &mut cpu6502) -> u8 {
        0
    }
    fn TYA(cpu: &mut cpu6502) -> u8 {
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
        }

        // Increment global clock count - This is actually unused unless logging is enabled
        // but I've kept it in because its a handy watch variable for debugging
        self.clock_count += 1;

        // Decrement the number of cycles remaining for this instruction
        self.cycles -= 1;
    }

    fn read(&self, address: u16) -> u8
    {
        self.bus.read(address, false)
    }

    fn write(&self, address: u16, value: u8)
    {
        self.write(address, value)
    }


    fn reset(&mut self)
    {
        // Get address to set program counter to
        self.addr_abs = 0xFFFC;
        let lo = self.read(self.addr_abs + 0);
        let hi = self.read(self.addr_abs + 1);

        // Set it
        self.pc = ((hi << 8) | lo) as u16;

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

    fn irq(&mut self)
    {
        if (self.get_flag(FLAGS6502::I) == 0)
        {
            // Push the program counter to the stack. It's 16-bits dont
            // forget so that takes two pushes
            self.write((0x0100u16 + self.stkp as u16) , ((self.pc >> 8) & 0x00FF) as u8);
            self.stkp -= 1;
            self.write((0x0100u16 + self.stkp as u16) , (self.pc & 0x00FF) as u8);
            self.stkp -= 1;

            // Then Push the status register to the stack
            self.set_flag(FLAGS6502::B, false);
            self.set_flag(FLAGS6502::U, true);
            self.set_flag(FLAGS6502::I, true);
            self.write(0x0100u16 + self.stkp as u16, self.status);
            self.stkp -= 1;

            // Read new program counter location from fixed address
            self.addr_abs = 0xFFFE;
            let lo = self.read(self.addr_abs + 0);
            let hi = self.read(self.addr_abs + 1);
            self.pc = ((hi << 8u16) | lo) as u16;

            // IRQs take time
            self.cycles = 7;
        }
    }

    fn nmi(&mut self)
    {
        self.write(0x0100u16 + self.stkp as u16, ((self.pc >> 8) & 0x00FF) as u8);
        self.stkp -= 1;
        self.write(0x0100u16 + self.stkp as u16, (self.pc & 0x00FF) as u8);
        self.stkp -= 1;

        self.set_flag(FLAGS6502::B, false);
        self.set_flag(FLAGS6502::U, true);
        self.set_flag(FLAGS6502::I, true);
        self.write(0x0100u16 + self.stkp as u16, self.status);
        self.stkp -= 1;

        self.addr_abs = 0xFFFA;
        let lo = self.read(self.addr_abs + 0);
        let hi = self.read(self.addr_abs + 1);
        self.pc = ((hi << 8) | lo) as u16;

        self.cycles = 8;
    }

    fn fetch() -> u8 {
        0
    }

    fn connect_bus(&mut self, bus: Rc<Bus>) {
        self.bus = bus
    }
}

fn main() {
    println!("Hello, world! {:?}", FLAGS6502::N as i8);
}
