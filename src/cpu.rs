use crate::control::{
    decode, exec_alu, exec_auipc, exec_branch, exec_load, exec_lui, exec_store, inst_i_imm,
    inst_rd, inst_rs1, inst_rs2, inst_u_imm,
};
// 保留最核心的组成：
// pc 程序计数器
// regs 32 个通用寄存器
// imem 指令内存
// dmem 按字节编址的数据内存
pub struct Cpu {
    pub pc: u32,
    pub regs: [u32; 32],
    pub imem: Vec<u32>,
    pub dmem: Vec<u8>,
}

impl Cpu {
    /// 创建一个 CPU 实例
    /// - PC 从 0 开始
    /// - 所有寄存器清零
    /// - 使用传入的指令内存
    /// - 数据内存初始化为 0
    pub fn new(imem: Vec<u32>, dmem_size: usize) -> Self {
        Self {
            pc: 0,
            regs: [0; 32],
            imem,
            dmem: vec![0; dmem_size],
        }
    }
}

impl Cpu {
    // 在当前 PC 位置执行一条指令
    pub fn step(&mut self) {
        // 将按字节编址的 PC 转成指令索引
        let idx = (self.pc / 4) as usize;

        // PC 越过已加载指令范围，直接停止
        if idx >= self.imem.len() {
            return;
        }

        // IF：取指
        let inst = self.imem[idx];

        // ID：解码控制字与寄存器字段
        let c = decode(inst);
        let rs1 = inst_rs1(inst) as usize;
        let rs2 = inst_rs2(inst) as usize;
        let rd = inst_rd(inst) as usize;
        let opcode = inst & 0x7f;

        // 默认顺序执行下一条，若控制流指令发生重定向则覆盖
        let mut next_pc = self.pc.wrapping_add(4);

        // 调用 opcode 得出的信号，按控制信号执行
        match opcode {
            // 算术逻辑指令：op rs1, rs2
            0x33 => {
                if let Some(op) = c.alu_op {
                    let out = exec_alu(op, self.regs[rs1], self.regs[rs2]);
                    if c.reg_write && rd != 0 {
                        self.regs[rd] = out;
                    }
                }
            }

            // 立即数算术逻辑指令：op rs1, imm32
            0x13 => {
                if let Some(op) = c.alu_op {
                    let imm = inst_i_imm(inst);
                    let out = exec_alu(op, self.regs[rs1], imm);
                    if c.reg_write && rd != 0 {
                        self.regs[rd] = out;
                    }
                }
            }

            // LOAD：有效地址 = rs1 + imm32，访存成功后写回 rd
            0x03 => {
                if let Some(kind) = c.load_kind {
                    let addr = self.regs[rs1].wrapping_add(inst_i_imm(inst));
                    if rd != 0 && let Some(val) = exec_load(kind, &self.dmem, addr) {
                        self.regs[rd] = val;
                    }
                }
            }

            // STORE：有效地址 = rs1 + imm32，写入数据来自 rs2
            0x23 => {
                if let Some(kind) = c.store_kind {
                    let imm = inst_s_imm(inst);
                    let addr = self.regs[rs1].wrapping_add(imm);
                    let _ = exec_store(kind, &mut self.dmem, addr, self.regs[rs2]);
                }
            }

            // LUI: rd = imm[31:12] << 12
            0x37 if c.reg_write && rd != 0 => {
                let imm = inst_u_imm(inst);
                self.regs[rd] = exec_lui(imm);
            }

            // AUIPC: rd = pc + imm[31:12] << 12
            0x17 if c.reg_write && rd != 0 => {
                let imm = inst_u_imm(inst);
                self.regs[rd] = exec_auipc(self.pc, imm);
            }

            // BRANCH: taken -> pc + b_imm; not taken -> pc + 4
            0x63 => {
                if let Some(kind) = c.branch_kind {
                    let taken = exec_branch(kind, self.regs[rs1], self.regs[rs2]);
                    if taken {
                        next_pc = self.pc.wrapping_add(inst_b_imm(inst));
                    }
                }
            }

            // 其它指令大类暂未接入
            _ => {}
        }

        // 提交 PC 更新
        self.pc = next_pc;

        // RISC-V 架构规则：x0 永远恒为 0
        self.regs[0] = 0;
    }
}

/// 提取 S-type 立即数并符号扩展到 32 位
fn inst_s_imm(inst: u32) -> u32 {
    let hi = (inst >> 25) & 0x7f;
    let lo = (inst >> 7) & 0x1f;
    let raw = (hi << 5) | lo;
    ((raw as i32) << 20 >> 20) as u32
}

/// 提取 B-type 立即数并符号扩展到 32 位
fn inst_b_imm(inst: u32) -> u32 {
    let imm12 = (inst >> 31) & 0x1;
    let imm11 = (inst >> 7) & 0x1;
    let imm10_5 = (inst >> 25) & 0x3f;
    let imm4_1 = (inst >> 8) & 0x0f;

    let raw = (imm12 << 12) | (imm11 << 11) | (imm10_5 << 5) | (imm4_1 << 1);
    ((raw as i32) << 19 >> 19) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 辅助函数：编码一条 R-type OP 指令
    fn encode_op(funct7: u32, rs2: u32, rs1: u32, funct3: u32, rd: u32) -> u32 {
        (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | 0x33
    }

    #[test]
    fn step_runs_addi() {
        // addi x2, x1, 5
        let addi_x2_x1_5 = 0x0050_8113u32;

        let mut cpu = Cpu::new(vec![addi_x2_x1_5], 0);
        cpu.regs[1] = 10;
        cpu.step();

        assert_eq!(cpu.regs[2], 15);
        assert_eq!(cpu.pc, 4);
    }

    #[test]
    fn step_runs_lui() {
        let lui_x5 = 0x123452b7u32;
        let mut cpu = Cpu::new(vec![lui_x5], 0);
        cpu.step();
        assert_eq!(cpu.regs[5], 0x12345000);
    }

    #[test]
    fn step_runs_auipc() {
        let auipc_x6 = 0x00001317u32;
        let mut cpu = Cpu::new(vec![0, auipc_x6], 0);
        cpu.pc = 4;
        cpu.step();
        assert_eq!(cpu.regs[6], 0x1004);
    }

    #[test]
    fn step_runs_add_then_sub_and_updates_pc() {
        let add_x3_x1_x2 = encode_op(0b0000000, 2, 1, 0b000, 3);
        let sub_x4_x3_x1 = encode_op(0b0100000, 1, 3, 0b000, 4);

        let mut cpu = Cpu::new(vec![add_x3_x1_x2, sub_x4_x3_x1], 0);
        cpu.regs[1] = 10;
        cpu.regs[2] = 7;

        cpu.step();
        cpu.step();

        assert_eq!(cpu.regs[3], 17);
        assert_eq!(cpu.regs[4], 7);
        assert_eq!(cpu.pc, 8);
    }

    #[test]
    fn x0_is_immutable() {
        let add_x0_x1_x2 = encode_op(0b0000000, 2, 1, 0b000, 0);

        let mut cpu = Cpu::new(vec![add_x0_x1_x2], 0);
        cpu.regs[1] = 123;
        cpu.regs[2] = 456;
        cpu.step();

        assert_eq!(cpu.regs[0], 0);
    }

    #[test]
    fn sw_then_lw_roundtrip() {
        // sw x2, 0(x1) -> 0x0020a023
        // lw x3, 0(x1) -> 0x0000a183
        let sw = 0x0020a023u32;
        let lw = 0x0000a183u32;

        let mut cpu = Cpu::new(vec![sw, lw], 64);
        cpu.regs[1] = 0;
        cpu.regs[2] = 0xdeadbeef;

        cpu.step();
        cpu.step();

        assert_eq!(cpu.regs[3], 0xdeadbeef);
    }
}
