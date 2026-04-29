/// 选择下一条 PC 的来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcSel {
    /// 顺序执行：PC + 4。
    Plus4,
    /// 分支目标地址。
    BranchTarget,
    /// 跳转目标地址。
    JumpTarget,
}

/// ALU 要执行的具体运算。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AluOp {
    Add,
    Sub,
    Sll,
    Slt,
    Sltu,
    Xor,
    Srl,
    Sra,
    Or,
    And,
}

//load执行类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadKind {
    Lb,
    Lh,
    Lw,
    Lbu,
    Lhu,
}

//store执行类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Storekind {
    SB,
    SH,
    SW,
}
/// 面向教学场景的最小控制字。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CtrlWord {
    // 允许写回寄存器堆。
    pub reg_write: bool,
    // 读取数据存储器。
    pub mem_read: bool,
    // 写入数据存储器。
    pub mem_write: bool,
    // ALU 第二操作数来自立即数。
    pub alu_src_imm: bool,
    // 条件分支类指令。
    pub branch: bool,
    // 无条件控制流跳转。
    pub jump: bool,
    // ALU 功能选择。
    pub alu_op: Option<AluOp>,
    //load指令种类
    pub load_kind: Option<LoadKind>, 
    //store指令种类
    pub store_kind: Option<Storekind>,
}

/// 解码 OP(0x33) 大类：R-type 算术逻辑指令。
fn decode_op(funct3: u32, funct7: u32) -> CtrlWord {
    let alu_op = match (funct3, funct7) {
        (0b000, 0b0000000) => Some(AluOp::Add),
        (0b000, 0b0100000) => Some(AluOp::Sub),
        (0b001, 0b0000000) => Some(AluOp::Sll),
        (0b010, 0b0000000) => Some(AluOp::Slt),
        (0b011, 0b0000000) => Some(AluOp::Sltu),
        (0b100, 0b0000000) => Some(AluOp::Xor),
        (0b101, 0b0000000) => Some(AluOp::Srl),
        (0b101, 0b0100000) => Some(AluOp::Sra),
        (0b110, 0b0000000) => Some(AluOp::Or),
        (0b111, 0b0000000) => Some(AluOp::And),
        _ => None,
    };

    match alu_op {
        Some(op) => CtrlWord {
            reg_write: true,
            alu_op: Some(op),
            ..Default::default()
        },
        None => CtrlWord::default(),
    }
}

//执行 OP 指令对应的 ALU 运算
pub fn exec_alu(op: AluOp, a: u32, b: u32) -> u32 {
    let shamt = b & 0x1f;
    match op {
        AluOp::Add => a.wrapping_add(b),
        AluOp::Sub => a.wrapping_sub(b),
        AluOp::Sll => a << shamt,
        AluOp::Slt => ((a as i32) < (b as i32)) as u32,
        AluOp::Sltu => (a < b) as u32,
        AluOp::Xor => a ^ b,
        AluOp::Srl => a >> shamt,
        AluOp::Sra => ((a as i32) >> shamt) as u32,
        AluOp::Or => a | b,
        AluOp::And => a & b,
    }
}

//解码load类
fn decode_load(funct3: u32) -> CtrlWord {
    let kind = match funct3 {
        0b000 => Some(LoadKind::Lb),
        0b001 => Some(LoadKind::Lh),
        0b010 => Some(LoadKind::Lw),
        0b100 => Some(LoadKind::Lbu),
        0b101 => Some(LoadKind::Lhu),
        _ => None,
    };

    match kind {
        Some(k) => CtrlWord {
            reg_write: true,
            mem_read: true,
            alu_src_imm: true,
            alu_op: Some(AluOp::Add), // rs1 + imm 算地址
            load_kind: Some(k),
            ..Default::default()
        },
        None => CtrlWord::default(),
    }
}

//执行load类
pub fn exec_load(kind: LoadKind, dmem: &[u8], addr: u32) -> Option<u32> {
    let a = addr as usize;   //将地址变成索引
    match kind {
        LoadKind::Lb => {
            let b = *dmem.get(a)?;   //取dmem的第a的字节
            Some((b as i8 as i32) as u32) // 加载字节，进行符号扩展
        }
        LoadKind::Lbu => {
            let b = *dmem.get(a)?;
            Some(b as u32) // 加载字节，无符号拓展，零扩展
        }
        LoadKind::Lh => {
            let bytes = dmem.get(a..a + 2)?; //读两个字节符号拓展
            let raw = u16::from_le_bytes([bytes[0], bytes[1]]);  //拼接字节
            Some((raw as i16 as i32) as u32) // 符号扩展
        }
        LoadKind::Lhu => {
            let bytes = dmem.get(a..a + 2)?;
            let raw = u16::from_le_bytes([bytes[0], bytes[1]]);
            Some(raw as u32) // 无符号拓展
        }
        LoadKind::Lw => {
            let bytes = dmem.get(a..a + 4)?;//读四个字节（一个字）
            Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        }
    }
}

//解码store类
fn decode_store(funct3: u32) -> CtrlWord {
    let kind = match funct3 {
        0b000 => Some(Storekind::SB),
        0b001 => Some(Storekind::SH),
        0b010 => Some(Storekind::SW),
        _ => None,
    };

    match kind {
        Some(k) => CtrlWord {
            mem_write: true,
            alu_src_imm: true,
            alu_op: Some(AluOp::Add),
            store_kind: Some(k),
            ..Default::default()
        },
        None => CtrlWord::default(),
    }
}

//执行store类
pub fn exec_store(kind: Storekind, dmem: &mut [u8], addr: u32, val: u32) -> Option<()> {
    let a = addr as usize;
    match kind {
        Storekind::SB => {
            *dmem.get_mut(a)? = val as u8; // 写最低字节   //把val低8位写入内存相应地址
            Some(())   //返回()表示操作成功
        }
        Storekind::SH => {
            let bytes = dmem.get_mut(a..a + 2)?;   //截取内存2个字节 
            let [lo, hi] = (val as u16).to_le_bytes();    //小端拆解存入lo,hi
            bytes[0] = lo;
            bytes[1] = hi;
            Some(())
        }
        Storekind::SW => {
            let bytes = dmem.get_mut(a..a + 4)?;
            let [b0, b1, b2, b3] = val.to_le_bytes();
            bytes[0] = b0;
            bytes[1] = b1;
            bytes[2] = b2;
            bytes[3] = b3;
            Some(())
        }
    }
}

//解码指令并生成控制信号，先进行大类指令区分，再针对 OP 指令进一步解码funct3和funct7
pub fn decode(inst: u32) -> CtrlWord {
    let opcode = inst & 0x7f;
    let funct3 = (inst >> 12) & 0x7;
    let funct7 = (inst >> 25) & 0x7f;

    match opcode {
        0x33 => decode_op(funct3, funct7), // OP大类
        0x03 => decode_load(funct3),       // LOAD大类
        0x23 => decode_store(funct3), // STORE大类
        0x63 => CtrlWord {
            branch: true,
            alu_op: Some(AluOp::Sub),
            ..Default::default()
        }, // BRANCH（当前按 beq 流程使用）
        0x6f => CtrlWord {
            reg_write: true,
            jump: true,
            ..Default::default()
        }, // JAL
        _ => CtrlWord::default(),
    }
}
//拆分二进制指令

/// 提取 rd 字段 [11:7]。
pub fn inst_rd(inst: u32) -> u8 {
    ((inst >> 7) & 0x1f) as u8
}

/// 提取 rs1 字段 [19:15]。
pub fn inst_rs1(inst: u32) -> u8 {
    ((inst >> 15) & 0x1f) as u8
}

/// 提取 rs2 字段 [24:20]。
pub fn inst_rs2(inst: u32) -> u8 {
    ((inst >> 20) & 0x1f) as u8
}

/// 流水线控制单元输出。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PipeCtrl {
    /// 冻结 IF（不更新 PC/IF 寄存器）。
    pub stall_if: bool,
    /// 冻结 ID（保持 IF/ID 寄存器）。
    pub stall_id: bool,
    /// 冲刷 EX（插入气泡）。
    pub flush_ex: bool,
    /// PC 重定向选择。
    pub pc_sel: Option<PcSel>,
}

/// 经典 5 级流水控制逻辑。
/// 优先级：
/// 1) load-use 冒险停顿。
/// 2) 控制转移冲刷。
pub fn hazard_unit(
    id_rs1: u8,
    id_rs2: u8,
    ex_rd: u8,
    ex_mem_read: bool,
    branch_taken: bool,
    jump: bool,
) -> PipeCtrl {
    let load_use = ex_mem_read && ex_rd != 0 && (ex_rd == id_rs1 || ex_rd == id_rs2);

    if load_use {
        return PipeCtrl {
            stall_if: true,
            stall_id: true,
            flush_ex: true,
            pc_sel: None,
        };
    }

    if branch_taken {
        return PipeCtrl {
            flush_ex: true,
            pc_sel: Some(PcSel::BranchTarget),
            ..Default::default()
        };
    }

    if jump {
        return PipeCtrl {
            flush_ex: true,
            pc_sel: Some(PcSel::JumpTarget),
            ..Default::default()
        };
    }

    PipeCtrl::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_op(funct7: u32, rs2: u32, rs1: u32, funct3: u32, rd: u32) -> u32 {
        (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | 0x33
    }

    #[test]
    fn decode_all_rv32i_op_variants() {
        let cases = [
            (0b000, 0b0000000, AluOp::Add),
            (0b000, 0b0100000, AluOp::Sub),
            (0b001, 0b0000000, AluOp::Sll),
            (0b010, 0b0000000, AluOp::Slt),
            (0b011, 0b0000000, AluOp::Sltu),
            (0b100, 0b0000000, AluOp::Xor),
            (0b101, 0b0000000, AluOp::Srl),
            (0b101, 0b0100000, AluOp::Sra),
            (0b110, 0b0000000, AluOp::Or),
            (0b111, 0b0000000, AluOp::And),
        ];

        for (funct3, funct7, expected) in cases {
            let inst = encode_op(funct7, 3, 2, funct3, 1);
            let c = decode(inst);
            assert!(c.reg_write);
            assert!(!c.mem_read);
            assert!(!c.mem_write);
            assert!(!c.alu_src_imm);
            assert!(!c.branch);
            assert!(!c.jump);
            assert_eq!(c.alu_op, Some(expected));
        }
    }

    #[test]
    fn invalid_op_encoding_is_nop() {
        let invalid = encode_op(0b0100000, 3, 2, 0b001, 1);
        let c = decode(invalid);
        assert_eq!(c, CtrlWord::default());
    }

    #[test]
    fn exec_alu_signed_unsigned_compare() {
        let a = 0xffff_ffff;
        let b = 1;
        assert_eq!(exec_alu(AluOp::Slt, a, b), 1);
        assert_eq!(exec_alu(AluOp::Sltu, a, b), 0);
    }

    #[test]
    fn exec_alu_shift_and_wrap_behavior() {
        assert_eq!(exec_alu(AluOp::Sra, 0x8000_0000, 1), 0xc000_0000);
        assert_eq!(exec_alu(AluOp::Sll, 1, 33), 2);
        assert_eq!(exec_alu(AluOp::Sub, 7, 10), 0xffff_fffd);
    }

    #[test]
    fn decode_lw_sw_beq_jal() {
        let lw = 0x00002083u32;
        let sw = 0x00102023u32;
        let beq = 0x00100063u32;
        let jal = 0x000000efu32;

        let lw_c = decode(lw);
        assert!(lw_c.reg_write && lw_c.mem_read && lw_c.alu_src_imm);
        assert_eq!(lw_c.alu_op, Some(AluOp::Add));

        let sw_c = decode(sw);
        assert!(sw_c.mem_write && sw_c.alu_src_imm);
        assert_eq!(sw_c.alu_op, Some(AluOp::Add));

        let beq_c = decode(beq);
        assert!(beq_c.branch);
        assert_eq!(beq_c.alu_op, Some(AluOp::Sub));

        let jal_c = decode(jal);
        assert!(jal_c.reg_write && jal_c.jump);
        assert_eq!(jal_c.alu_op, None);
    }

    #[test]
    fn extract_reg_fields() {
        let inst = 0b0000000_00101_00100_000_00011_0110011u32;
        assert_eq!(inst_rd(inst), 3);
        assert_eq!(inst_rs1(inst), 4);
        assert_eq!(inst_rs2(inst), 5);
    }

    #[test]
    fn load_use_hazard_stalls_pipeline() {
        let p = hazard_unit(5, 7, 5, true, false, false);
        assert_eq!(
            p,
            PipeCtrl {
                stall_if: true,
                stall_id: true,
                flush_ex: true,
                pc_sel: None
            }
        );
    }

    #[test]
    fn branch_taken_flushes_and_redirects_pc() {
        let p = hazard_unit(1, 2, 0, false, true, false);
        assert_eq!(
            p,
            PipeCtrl {
                stall_if: false,
                stall_id: false,
                flush_ex: true,
                pc_sel: Some(PcSel::BranchTarget)
            }
        );
    }

    #[test]
    fn jump_flushes_and_redirects_pc() {
        let p = hazard_unit(1, 2, 0, false, false, true);
        assert_eq!(
            p,
            PipeCtrl {
                stall_if: false,
                stall_id: false,
                flush_ex: true,
                pc_sel: Some(PcSel::JumpTarget)
            }
        );
    }

    #[test]
    fn load_use_has_higher_priority_than_branch() {
        let p = hazard_unit(8, 9, 8, true, true, false);
        assert!(p.stall_if);
        assert!(p.stall_id);
        assert!(p.flush_ex);
        assert_eq!(p.pc_sel, None);
    }
}
