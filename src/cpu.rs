use crate::control::{decode, exec_alu, inst_rd, inst_rs1, inst_rs2};

pub struct Cpu {      //PC,32位32个寄存器，指令内存
    pub pc: u32,                     
    pub regs: [u32; 32],
    pub imem: Vec<u32>,
}

impl Cpu {
    pub fn new(imem: Vec<u32>) -> Self {    //初始化PC和寄存器，加载指令内存，导入指令
        Self {
            pc: 0,
            regs: [0; 32],
            imem,
        }
    }
}

impl Cpu {
    pub fn step(&mut self) {
        let idx = (self.pc / 4) as usize;           //每条指令四个字节
        if idx >= self.imem.len() { //PC超出内存大小，停止
            return;
        }

        let inst = self.imem[idx];   //提取二进制指令
        let c = decode(inst);  //调用解码函数
        let rs1 = inst_rs1(inst) as usize;   //第一个源寄存器索引
        let rs2 = inst_rs2(inst) as usize;   //第二个源寄存器索引
        let rd = inst_rd(inst) as usize;    //目的寄存器索引

        if let Some(op) = c.alu_op {
            let out = exec_alu(op, self.regs[rs1], self.regs[rs2]);    //调用ALU函数求结果
            if c.reg_write && rd != 0 {      //状态寄存器写使能且目的寄存器不是x0，可以写则写回结果
                self.regs[rd] = out;  
            }
        }

        self.pc = self.pc.wrapping_add(4);   //PC自增到下一条指令
        self.regs[0] = 0;    //恢复x0寄存器为0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_op(funct7: u32, rs2: u32, rs1: u32, funct3: u32, rd: u32) -> u32 {
        (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | 0x33
    }

    #[test]
    fn step_runs_add_then_sub_and_updates_pc() {
        let add_x3_x1_x2 = encode_op(0b0000000, 2, 1, 0b000, 3);
        let sub_x4_x3_x1 = encode_op(0b0100000, 1, 3, 0b000, 4);

        let mut cpu = Cpu::new(vec![add_x3_x1_x2, sub_x4_x3_x1]);
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

        let mut cpu = Cpu::new(vec![add_x0_x1_x2]);
        cpu.regs[1] = 123;
        cpu.regs[2] = 456;

        cpu.step();
        assert_eq!(cpu.regs[0], 0);
    }
}
