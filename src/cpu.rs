use crate::control::{
    decode, exec_alu, exec_auipc, exec_branch, exec_jal, exec_jalr, exec_load, exec_lui,
    exec_store, inst_i_imm, inst_jal_imm, inst_jalr_imm, inst_rd, inst_rs1, inst_rs2, inst_u_imm,
};


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
    /// PC 从 0 开始
    /// 所有寄存器清零
    /// 使用传入的指令内存
    /// 数据内存初始化为 0
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
        let funct3 = (inst >> 12) & 0x7;
        //let funct7 = (inst >> 25) & 0x7f;

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
                    if rd != 0
                        && let Some(val) = exec_load(kind, &self.dmem, addr)
                    {
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
            // JAL: rd = pc + 4; pc = pc + imm
            0x6f => {
                let imm = inst_jal_imm(inst);
                let (target, ret) = exec_jal(self.pc, imm);
                if c.reg_write && rd != 0 {
                    self.regs[rd] = ret;
                }
                next_pc = target;
            }

            // JALR: rd = pc + 4; pc = (rs1 + imm) & !1
            0x67 if c.jump => {
                let imm = inst_jalr_imm(inst); // 这里用 inst_i_imm(inst) 也可以
                let (target, ret) = exec_jalr(self.regs[rs1], imm, self.pc);
                if c.reg_write && rd != 0 {
                    self.regs[rd] = ret;
                }
                next_pc = target;
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
