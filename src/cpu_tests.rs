#[cfg(test)]
mod tests {
    use crate::cpu::Cpu;

    /// branch指令拼接
    fn encode_branch(rs1: u32, rs2: u32, funct3: u32, imm: u32) -> u32 {
        let imm12 = (imm >> 12) & 0x1;
        let imm10_5 = (imm >> 5) & 0x3f;
        let imm4_1 = (imm >> 1) & 0xf;
        let imm11 = (imm >> 11) & 0x1;
        (imm12 << 31)
            | (imm10_5 << 25)
            | (rs2 << 20)
            | (rs1 << 15)
            | (funct3 << 12)
            | (imm4_1 << 8)
            | (imm11 << 7)
            | 0x63
    }

    // load指令拼接
    fn encode_load(rd: u32, rs1: u32, funct3: u32, imm: u32) -> u32 {
        ((imm & 0xfff) << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | 0x03
    }

    // store指令拼接
    fn encode_store(rs1: u32, rs2: u32, funct3: u32, imm: u32) -> u32 {
        let imm11_5 = (imm >> 5) & 0x7f;
        let imm4_0 = imm & 0x1f;

        (imm11_5 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (imm4_0 << 7) | 0x23
    }

    /// 辅助函数：编码一条 R-type OP 指令
    fn encode_op(funct7: u32, rs2: u32, rs1: u32, funct3: u32, rd: u32) -> u32 {
        (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | 0x33
    }

    #[test]
    // 检查sb低8位存放
    fn step_runs_sb_store_low_byte() {
        let sb = encode_store(1, 2, 0b000, 0);
        let mut cpu = Cpu::new(vec![sb], 8);

        cpu.regs[1] = 0;
        cpu.regs[2] = 0x1234_5678;

        cpu.step();

        assert_eq!(cpu.dmem[0], 0x78);
    }

    #[test]
    // 检查sh低16位存放
    fn step_runs_sh_store_low_byte() {
        let sh = encode_store(1, 2, 0b001, 0);
        let mut cpu = Cpu::new(vec![sh], 8);

        cpu.regs[1] = 0;
        cpu.regs[2] = 0x1234_5678;

        cpu.step();

        assert_eq!(cpu.dmem[0], 0x78);
        assert_eq!(cpu.dmem[1], 0x56);
    }

    #[test]
    // 检查sw低32位存放
    fn step_runs_sw_store_low_byte() {
        let sw = encode_store(1, 2, 0b010, 0);
        let mut cpu = Cpu::new(vec![sw], 16);

        cpu.regs[1] = 0;
        cpu.regs[2] = 0x1234_5678;

        cpu.step();

        assert_eq!(cpu.dmem[0], 0x78);
        assert_eq!(cpu.dmem[1], 0x56);
        assert_eq!(cpu.dmem[2], 0x34);
        assert_eq!(cpu.dmem[3], 0x12);
    }

    #[test]
    /// 检查lb时的符号拓展
    fn step_runs_lb_sign_extend() {
        let lb = encode_load(3, 1, 0b000, 0);
        let mut cpu = Cpu::new(vec![lb], 8);

        cpu.regs[1] = 0;
        cpu.dmem[0] = 0xff;

        cpu.step();

        assert_eq!(cpu.regs[3], 0xffff_ffff);
    }

    #[test]
    /// 检查lbu时的零拓展
    fn step_runs_lbu_sign_extend() {
        let lbu = encode_load(3, 1, 0b100, 0);
        let mut cpu = Cpu::new(vec![lbu], 8);

        cpu.regs[1] = 0;
        cpu.dmem[0] = 0xff;

        cpu.step();

        assert_eq!(cpu.regs[3], 0x0000_00ff);
    }

    #[test]
    /// 检查lh的符号拓展
    fn step_runs_lh_sign_extend() {
        let lh = encode_load(3, 1, 0b001, 0);
        let mut cpu = Cpu::new(vec![lh], 16);

        cpu.regs[1] = 0;
        cpu.dmem[0] = 0xff;
        cpu.dmem[1] = 0xff;

        cpu.step();

        assert_eq!(cpu.regs[3], 0xffff_ffff);
    }

    #[test]
    /// 检查lhu的零拓展
    fn step_runs_lhu_zero_extend() {
        let lhu = encode_load(3, 1, 0b101, 0);
        let mut cpu = Cpu::new(vec![lhu], 16);

        cpu.regs[1] = 0;
        cpu.dmem[0] = 0xff;
        cpu.dmem[1] = 0xff;

        cpu.step();

        assert_eq!(cpu.regs[3], 0x0000_ffff);
    }

    #[test]
    fn step_runs_beq_taken() {
        let beq = encode_branch(1, 2, 0b000, 8);
        let mut cpu = Cpu::new(vec![beq], 0);

        cpu.regs[1] = 7;
        cpu.regs[2] = 7;

        cpu.step();

        assert_eq!(cpu.pc, 8);
    }

    #[test]
    fn step_runs_beq_not_taken() {
        let beq = encode_branch(1, 2, 0b000, 8);
        let mut cpu = Cpu::new(vec![beq], 0);

        cpu.regs[1] = 7;
        cpu.regs[2] = 9;

        cpu.step();

        assert_eq!(cpu.pc, 4);
    }

    #[test]
    fn step_runs_beq_taken_nagtive_imm() {
        let beq = encode_branch(1, 2, 0b000, (-4i32) as u32);
        let mut cpu = Cpu::new(vec![0, 0, 0, 0, beq], 0);
        // 因为需要PC=16，则用[0,0,0,0]当作指令占位数
        cpu.pc = 16;
        cpu.regs[1] = 7;
        cpu.regs[2] = 7;

        cpu.step();

        assert_eq!(cpu.pc, 12);
    }

    #[test]
    fn run_jalr_with_funct3not0_and_clear_lsb() {
        let jalr_x5_x1_0 = 0x0000_f2e7u32;
        let mut cpu = Cpu::new(vec![jalr_x5_x1_0], 0);
        cpu.regs[1] = 9; // 目标=9，执行后应清零最低位到 8

        cpu.step();

        assert_eq!(cpu.regs[5], 0); // 返回地址
        assert_eq!(cpu.pc, 4); // (9 + 0) & !1
    }

    #[test]
    fn step_runs_jal_and_writes_ra() {
        // jal x1, +8
        let jal_x1_plus8 = 0x0080_00efu32;
        let mut cpu = Cpu::new(vec![jal_x1_plus8], 0);

        cpu.step();

        assert_eq!(cpu.regs[1], 4); // 返回地址
        assert_eq!(cpu.pc, 8); // 跳到 PC+8
    }

    #[test]
    fn step_runs_jalr_and_clears_lsb() {
        // jalr x5, x1, 0
        let jalr_x5_x1_0 = 0x0000_82e7u32;
        let mut cpu = Cpu::new(vec![jalr_x5_x1_0], 0);
        cpu.regs[1] = 9; // 目标=9，执行后应清零最低位到 8

        cpu.step();

        assert_eq!(cpu.regs[5], 4); // 返回地址
        assert_eq!(cpu.pc, 8); // (9 + 0) & !1
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
