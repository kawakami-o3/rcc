use crate::regalloc::*;
use crate::*;

lazy_static! {
    static ref LABEL: Mutex<usize> = Mutex::new(0);
}

fn label() -> usize {
    *LABEL.lock().unwrap()
}

fn inc_label() {
    let mut label = LABEL.lock().unwrap();
    *label += 1;
}

fn gen(fun: &IR) {
    let ret = format!(".Lend{}", label());
    inc_label();

    println!(".global {}", fun.name);
    println!("{}:", fun.name);
    println!("  push rbp");
    println!("  mov rbp, rsp");
    println!("  sub rsp, {}", fun.stacksize);
    println!("  push r12");
    println!("  push r13");
    println!("  push r14");
    println!("  push r15");

    let regs = REGS.lock().unwrap();
    for i in 0..fun.ir.len() {
        let ir = &fun.ir[i as usize];
        match ir.op {
            IRType::IMM => {
                println!("  mov {}, {}", regs[ir.lhs as usize], ir.rhs);
            }
            IRType::ADD_IMM => {
                println!("  add {}, {}", regs[ir.lhs as usize], ir.rhs);
            }
            IRType::MOV => {
                println!("  mov {}, {}", regs[ir.lhs as usize], regs[ir.rhs as usize]);
            }
            IRType::RETURN => {
                println!("  mov rax, {}", regs[ir.lhs as usize]);
                println!("  jmp {}", ret);
            }
            IRType::CALL => {
                println!("  push rbx");
                println!("  push rbp");
                println!("  push rsp");
                println!("  push r12");
                println!("  push r13");
                println!("  push r14");
                println!("  push r15");

                let arg = vec!["rdi", "rsi", "rdx", "rcx", "r8", "r9"];
                for i in 0..arg.len() {
                    println!("  mov {}, {}", arg[i], regs[ir.args[i] as usize]);
                }

                println!("  push r10");
                println!("  push r11");
                println!("  mov rax, 0");
                println!("  call {}", ir.name);
                println!("  pop r11");
                println!("  pop r10");

                println!("  mov {}, rax", regs[ir.lhs as usize]);
            }
            IRType::LABEL => {
                println!(".L{}:", ir.lhs);
            }
            IRType::JMP => {
                println!("  jmp .L{}", ir.lhs);
            }
            IRType::UNLESS => {
                println!("  cmp {}, 0", regs[ir.lhs as usize]);
                println!("  je .L{}", ir.rhs);
            }
            IRType::LOAD => {
                println!(
                    "  mov {}, [{}]",
                    regs[ir.lhs as usize], regs[ir.rhs as usize]
                );
            }
            IRType::STORE => {
                println!(
                    "  mov [{}], {}",
                    regs[ir.lhs as usize], regs[ir.rhs as usize]
                );
            }
            IRType::ADD => {
                println!("  add {}, {}", regs[ir.lhs as usize], regs[ir.rhs as usize]);
            }
            IRType::SUB => {
                println!("  sub {}, {}", regs[ir.lhs as usize], regs[ir.rhs as usize]);
            }
            IRType::MUL => {
                println!("  mov rax, {}", regs[ir.rhs as usize]);
                println!("  mul {}", regs[ir.lhs as usize]);
                println!("  mov {}, rax", regs[ir.lhs as usize]);
            }
            IRType::DIV => {
                println!(" mov rax, {}", regs[ir.lhs as usize]);
                println!(" cqo");
                println!(" div {}", regs[ir.rhs as usize]);
                println!(" mov {}, rax", regs[ir.lhs as usize]);
            }
            IRType::NOP => {}
            ref i => {
                panic!(format!("unknown operator {:?}", i));
            }
        }
    }
    println!("{}:", ret);
    println!("  pop r15");
    println!("  pop r14");
    println!("  pop r13");
    println!("  pop r12");
    println!("  mov rsp, rbp");
    println!("  pop rbp");
    println!("  ret");
}


pub fn gen_x86(fns: &Vec<IR>) {
    println!(".intel_syntax noprefix");

    for i in 0..fns.len() {
        gen(&fns[i]);
    }
}
