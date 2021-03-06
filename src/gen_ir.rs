// Intermediate representation

// 9cc's code generation is two-pass. In the first pass, abstract
// syntax trees are compiled to IR (intermediate representation).
//
// IR resembles the real x86-64 instruction set, but it has infinite
// number of registers. We don't try too hard to reuse registers in
// this pass. Instead, we "kill" registers to mark them as dead when
// we are done with them and use new registers.
//
// Such infinite number of registers are mapped to a finite registers
// in a later pass.

// let mut off = 0;
// for v in func.lvars.iter_mut() {
//     off = roundup(off, v.borrow().ty.align);
//     off += v.borrow().ty.size;
//     v.borrow_mut().offset = off;
// }
// func.stacksize = off;

#![allow(non_camel_case_types)]

use crate::parse::*;
use crate::util::*;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static FN: RefCell<Option<Rc<RefCell<Function>>>> = RefCell::new(None);
    static OUT: RefCell<Option<Rc<RefCell<BB>>>> = RefCell::new(None);
    static NREG: RefCell<i32> = RefCell::new(1);
    static BREAK_LABEL: RefCell<i32> = RefCell::new(0);
}

fn set_fn(fun: Rc<RefCell<Function>>) {
    FN.with(|f| {
        *f.borrow_mut() = Some(fun);
    })
}

fn fn_bbs_push(bb: Rc<RefCell<BB>>) {
    FN.with(|f| match *f.borrow() {
        Some(ref fun) => {
            fun.borrow_mut().bbs.push(bb);
        }
        None => {
            panic!();
        }
    })
}

fn set_out(bb: Rc<RefCell<BB>>) {
    OUT.with(|o| {
        *o.borrow_mut() = Some(bb);
    })
}

fn out_ir_push(ir: Rc<RefCell<IR>>) {
    OUT.with(|o| match *o.borrow() {
        Some(ref out) => {
            out.borrow_mut().ir.push(ir);
        }
        None => {
            panic!();
        }
    })
}

fn out_param_set(reg: Rc<RefCell<Reg>>) {
    OUT.with(|o| match *o.borrow() {
        Some(ref out) => {
            out.borrow_mut().param = Some(reg);
        }
        None => {
            panic!();
        }
    })
}

fn out_param_get() -> Rc<RefCell<Reg>> {
    OUT.with(|o| match *o.borrow() {
        Some(ref out) => {
            return out.borrow_mut().param.clone().unwrap();
        }
        None => {
            panic!();
        }
    })
}

fn bump_nreg() -> i32 {
    NREG.with(|v| {
        let ret = *v.borrow();
        *v.borrow_mut() += 1;
        return ret;
    })
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum IRType {
    IMM,
    BPREL,
    MOV,
    RETURN,
    CALL,
    LABEL_ADDR,
    EQ,
    NE,
    LE,
    LT,
    AND,
    OR,
    XOR,
    SHL,
    SHR,
    MOD,
    JMP,
    BR,
    LOAD,
    LOAD_SPILL,
    STORE,
    STORE_ARG,
    STORE_SPILL,
    ADD,
    SUB,
    MUL,
    DIV,
    NOP,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reg {
    pub vn: i32, // virtual register number
    pub rn: i32, // real register number

    // For optimizer
    pub promoted: Option<Rc<RefCell<Reg>>>,

    // For regalloc
    pub def: i32,
    pub last_use: i32,
    pub spill: bool,
    pub var: Option<Rc<RefCell<Var>>>,
}

fn alloc_reg() -> Rc<RefCell<Reg>> {
    Rc::new(RefCell::new(Reg {
        vn: -1,
        rn: -1,

        promoted: None,

        def: -1,
        last_use: -1,
        spill: false,
        var: None,
    }))
}

#[derive(Clone, Debug, PartialEq)]
pub struct BB {
    pub label: usize,
    pub ir: Vec<Rc<RefCell<IR>>>,
    pub param: Option<Rc<RefCell<Reg>>>,

    pub succ: Vec<Rc<RefCell<BB>>>,
    pub pred: Vec<Rc<RefCell<BB>>>,
    pub def_regs: Rc<RefCell<Vec<Rc<RefCell<Reg>>>>>,
    pub in_regs: Rc<RefCell<Vec<Rc<RefCell<Reg>>>>>,
    pub out_regs: Rc<RefCell<Vec<Rc<RefCell<Reg>>>>>,
}

pub fn alloc_bb() -> BB {
    BB {
        label: 0,
        ir: Vec::new(),
        param: None,

        succ: Vec::new(),
        pred: Vec::new(),
        def_regs: Rc::new(RefCell::new(Vec::new())),
        in_regs: Rc::new(RefCell::new(Vec::new())),
        out_regs: Rc::new(RefCell::new(Vec::new())),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IR {
    pub op: IRType,

    pub r0: Option<Rc<RefCell<Reg>>>,
    pub r1: Option<Rc<RefCell<Reg>>>,
    pub r2: Option<Rc<RefCell<Reg>>>,

    pub imm: i32,
    pub label: i32,
    pub var: Option<Rc<RefCell<Var>>>,

    pub bb1: Option<Rc<RefCell<BB>>>,
    pub bb2: Option<Rc<RefCell<BB>>>,

    // Load/store size in bytes
    pub size: i32,

    // Function call
    pub name: String,
    pub nargs: usize,
    pub args: Vec<Rc<RefCell<Reg>>>,

    // Function struct fields in 9cc
    pub stacksize: i32,
    pub ir: Vec<IR>,
    pub globals: Vec<Var>,

    // For liveness tracking
    pub kill: Vec<Rc<RefCell<Reg>>>,

    // For SSA
    pub bbarg: Option<Rc<RefCell<Reg>>>,
}

pub fn alloc_ir() -> IR {
    IR {
        op: IRType::NOP,

        r0: None,
        r1: None,
        r2: None,

        imm: 0,
        label: 0,
        var: None,

        bb1: None,
        bb2: None,

        size: 0,

        name: String::new(),
        nargs: 0,
        args: Vec::new(),

        stacksize: 0,
        ir: Vec::new(),
        globals: Vec::new(),

        kill: Vec::new(),

        bbarg: None,
    }
}

fn new_bb() -> Rc<RefCell<BB>> {
    let mut bb = alloc_bb();
    bb.label = bump_nlabel();
    bb.ir = Vec::new();
    bb.succ = Vec::new();
    bb.pred = Vec::new();
    bb.def_regs = Rc::new(RefCell::new(Vec::new()));
    bb.in_regs = Rc::new(RefCell::new(Vec::new()));
    bb.out_regs = Rc::new(RefCell::new(Vec::new()));
    let b = Rc::new(RefCell::new(bb));
    fn_bbs_push(b.clone());
    return b;
}

fn new_ir(op: IRType) -> Rc<RefCell<IR>> {
    let mut ir = alloc_ir();
    ir.op = op;
    let i = Rc::new(RefCell::new(ir));
    out_ir_push(i.clone());
    return i;
}

pub fn new_reg() -> Rc<RefCell<Reg>> {
    let r = alloc_reg();
    r.borrow_mut().vn = bump_nreg();
    r.borrow_mut().rn = -1;
    return r;
}

fn emit(
    op: IRType,
    r0: Option<Rc<RefCell<Reg>>>,
    r1: Option<Rc<RefCell<Reg>>>,
    r2: Option<Rc<RefCell<Reg>>>,
) -> Rc<RefCell<IR>> {
    let ir = new_ir(op);
    ir.borrow_mut().r0 = r0;
    ir.borrow_mut().r1 = r1;
    ir.borrow_mut().r2 = r2;
    return ir;
}

fn br(r: Rc<RefCell<Reg>>, then: Rc<RefCell<BB>>, els: Rc<RefCell<BB>>) -> Rc<RefCell<IR>> {
    let ir = new_ir(IRType::BR);
    ir.borrow_mut().r2 = Some(r);
    ir.borrow_mut().bb1 = Some(then);
    ir.borrow_mut().bb2 = Some(els);
    return ir;
}

fn jmp(bb: Rc<RefCell<BB>>) -> Rc<RefCell<IR>> {
    let ir = new_ir(IRType::JMP);
    ir.borrow_mut().bb1 = Some(bb);
    return ir;
}

fn jmp_arg(bb: Rc<RefCell<BB>>, r: Rc<RefCell<Reg>>) -> Rc<RefCell<IR>> {
    let ir = new_ir(IRType::JMP);
    ir.borrow_mut().bb1 = Some(bb);
    ir.borrow_mut().bbarg = Some(r);
    return ir;
}

fn imm(imm: i32) -> Rc<RefCell<Reg>> {
    let r = new_reg();
    let ir = new_ir(IRType::IMM);
    ir.borrow_mut().r0 = Some(r.clone());
    ir.borrow_mut().imm = imm;
    return r;
}

fn load(node: Rc<RefCell<Node>>, dst: Rc<RefCell<Reg>>, src: Rc<RefCell<Reg>>) {
    let ir = emit(IRType::LOAD, Some(dst), None, Some(src));
    let ty = node.borrow().ty.clone();
    ir.borrow_mut().size = ty.borrow().size;
}

// In C, all expressions that can be written on the left-hand side of
// the '=' operator must have an address in memory. In other words, if
// you can apply the '&' operator to take an address of some
// expression E, you can assign E to a new value.
//
// Other expressions, such as `1+2`, cannot be written on the lhs of
// '=', since they are just temporary values that don't have an address.
//
// The stuff that can be written on the lhs of '=' is called lvalue.
// Other values are called rvalue. An lvalue is essentially an address.
//
// When lvalues appear on the rvalue context, they are converted to
// rvalues by loading their values from their addresses. You can think
// '&' as an operator that suppresses such automatic lvalue-to-rvalue
// conversion.
//
// This function evaluates a given node as an lvalue.
fn gen_lval(node: Rc<RefCell<Node>>) -> Rc<RefCell<Reg>> {
    let node_op = node.borrow().op.clone();
    if node_op == NodeType::DEREF {
        return gen_expr(node.borrow().expr.clone().unwrap());
    }

    if node_op == NodeType::DOT {
        let ty = node.borrow().ty.clone();
        let r1 = new_reg();
        let r2 = gen_lval(node.borrow().expr.clone().unwrap());
        let r3 = imm(ty.borrow().offset);
        emit(
            IRType::ADD,
            Some(r1.clone()),
            Some(r2.clone()),
            Some(r3.clone()),
        );
        return r1;
    }

    assert!(node_op == NodeType::VARREF);
    let var = node.borrow().var.clone().unwrap();

    let ir: Rc<RefCell<IR>>;
    if var.borrow().is_local {
        ir = new_ir(IRType::BPREL);
        ir.borrow_mut().r0 = Some(new_reg());
        ir.borrow_mut().var = Some(var);
    } else {
        ir = new_ir(IRType::LABEL_ADDR);
        ir.borrow_mut().r0 = Some(new_reg());
        ir.borrow_mut().name = var.borrow().name.clone();
    }
    return ir.borrow().r0.clone().unwrap();
}

fn gen_binop(ty: IRType, node: Rc<RefCell<Node>>) -> Rc<RefCell<Reg>> {
    let r1 = new_reg();
    let r2 = gen_expr(node.borrow().lhs.clone().unwrap());
    let r3 = gen_expr(node.borrow().rhs.clone().unwrap());
    emit(ty, Some(r1.clone()), Some(r2.clone()), Some(r3.clone()));
    return r1;
}

fn gen_expr(node: Rc<RefCell<Node>>) -> Rc<RefCell<Reg>> {
    let op = node.borrow().op.clone();
    match op {
        NodeType::NUM => {
            return imm(node.borrow().val);
        }

        NodeType::EQ => {
            return gen_binop(IRType::EQ, node);
        }

        NodeType::NE => {
            return gen_binop(IRType::NE, node);
        }

        NodeType::LOGAND => {
            let bb = new_bb();
            let set0 = new_bb();
            let set1 = new_bb();
            let last = new_bb();

            br(
                gen_expr(node.borrow().lhs.clone().unwrap()),
                bb.clone(),
                set0.clone(),
            );

            set_out(bb);
            br(
                gen_expr(node.borrow().rhs.clone().unwrap()),
                set1.clone(),
                set0.clone(),
            );

            set_out(set0);
            jmp_arg(last.clone(), imm(0));

            set_out(set1);
            jmp_arg(last.clone(), imm(1));

            set_out(last);
            out_param_set(new_reg());
            return out_param_get();
        }

        NodeType::LOGOR => {
            let bb = new_bb();
            let set0 = new_bb();
            let set1 = new_bb();
            let last = new_bb();

            let r1 = gen_expr(node.borrow().lhs.clone().unwrap());
            br(r1.clone(), set1.clone(), bb.clone());

            set_out(bb.clone());
            let r2 = gen_expr(node.borrow().rhs.clone().unwrap());
            br(r2.clone(), set1.clone(), set0.clone());

            set_out(set0.clone());
            jmp_arg(last.clone(), imm(0));

            set_out(set1.clone());
            jmp_arg(last.clone(), imm(1));

            set_out(last);
            out_param_set(new_reg());
            return out_param_get();
        }

        NodeType::VARREF | NodeType::DOT => {
            let r = new_reg();
            load(node.clone(), r.clone(), gen_lval(node.clone()));
            return r;
        }

        NodeType::CALL => {
            let mut args = Vec::new();
            for a in node.borrow().args.iter() {
                args.push(gen_expr(a.clone()));
            }

            let ir = new_ir(IRType::CALL);
            ir.borrow_mut().r0 = Some(new_reg());
            ir.borrow_mut().name = node.borrow().name.clone();
            ir.borrow_mut().nargs = node.borrow().args.len();
            let nargs = ir.borrow().nargs;
            for i in 0..nargs {
                ir.borrow_mut().args.push(args[i].clone());
            }
            return ir.borrow().r0.clone().unwrap();
        }

        NodeType::ADDR => {
            return gen_lval(node.borrow().expr.clone().unwrap());
        }

        NodeType::DEREF => {
            let r = new_reg();
            load(
                node.clone(),
                r.clone(),
                gen_expr(node.borrow().expr.clone().unwrap()),
            );
            return r;
        }

        NodeType::CAST => {
            let r1 = gen_expr(node.borrow().expr.clone().unwrap());
            let ty = node.borrow().ty.clone();
            if ty.borrow().ty != CType::BOOL {
                return r1;
            }
            let r2 = new_reg();
            emit(IRType::NE, Some(r2.clone()), Some(r1.clone()), Some(imm(0)));
            return r2;
        }

        NodeType::STMT_EXPR => {
            for n in node.borrow().stmts.iter() {
                gen_stmt(n.clone());
            }
            return gen_expr(node.borrow().expr.clone().unwrap());
        }

        NodeType::EQL => {
            let r1 = gen_expr(node.borrow().rhs.clone().unwrap());
            let r2 = gen_lval(node.borrow().lhs.clone().unwrap());

            let ir = emit(IRType::STORE, None, Some(r2.clone()), Some(r1.clone()));
            let ty = node.borrow().ty.clone();
            ir.borrow_mut().size = ty.borrow().size;
            return r1;
        }
        NodeType::ADD => {
            return gen_binop(IRType::ADD, node);
        }
        NodeType::SUB => {
            return gen_binop(IRType::SUB, node);
        }
        NodeType::MUL => {
            return gen_binop(IRType::MUL, node);
        }
        NodeType::DIV => {
            return gen_binop(IRType::DIV, node);
        }
        NodeType::MOD => {
            return gen_binop(IRType::MOD, node);
        }
        NodeType::LT => {
            return gen_binop(IRType::LT, node);
        }
        NodeType::LE => {
            return gen_binop(IRType::LE, node);
        }
        NodeType::AND => {
            return gen_binop(IRType::AND, node);
        }
        NodeType::OR => {
            return gen_binop(IRType::OR, node);
        }
        NodeType::XOR => {
            return gen_binop(IRType::XOR, node);
        }
        NodeType::SHL => {
            return gen_binop(IRType::SHL, node);
        }
        NodeType::SHR => {
            return gen_binop(IRType::SHR, node);
        }
        NodeType::NOT => {
            let r1 = new_reg();
            let r2 = gen_expr(node.borrow().expr.clone().unwrap());
            emit(
                IRType::XOR,
                Some(r1.clone()),
                Some(r2.clone()),
                Some(imm(-1)),
            );
            return r1;
        }
        NodeType::COMMA => {
            gen_expr(node.borrow().lhs.clone().unwrap());
            return gen_expr(node.borrow().rhs.clone().unwrap());
        }
        NodeType::QUEST => {
            let then = new_bb();
            let els = new_bb();
            let last = new_bb();

            br(
                gen_expr(node.borrow().cond.clone().unwrap()),
                then.clone(),
                els.clone(),
            );

            set_out(then);
            jmp_arg(last.clone(), gen_expr(node.borrow().then.clone().unwrap()));

            set_out(els);
            jmp_arg(last.clone(), gen_expr(node.borrow().els.clone().unwrap()));

            set_out(last);
            out_param_set(new_reg());
            return out_param_get();
        }
        NodeType::EXCLAM => {
            let r1 = new_reg();
            let r2 = gen_expr(node.borrow().expr.clone().unwrap());
            emit(IRType::EQ, Some(r1.clone()), Some(r2.clone()), Some(imm(0)));
            return r1;
        }
        t => {
            panic!("unknown AST type {:?}", t);
        }
    }
}

fn gen_stmt(node: Rc<RefCell<Node>>) {
    if node.borrow().op == NodeType::NULL {
        return;
    }

    let op = node.borrow().op.clone();
    match op {
        NodeType::IF => {
            let then = new_bb();
            let els = new_bb();
            let last = new_bb();

            br(
                gen_expr(node.borrow().cond.clone().unwrap()),
                then.clone(),
                els.clone(),
            );

            set_out(then);
            gen_stmt(node.borrow().then.clone().unwrap());
            jmp(last.clone());

            set_out(els);
            if node.borrow().els.is_some() {
                gen_stmt(node.borrow().els.clone().unwrap());
            }
            jmp(last.clone());

            set_out(last);
        }
        NodeType::FOR => {
            let cond = new_bb();
            node.borrow_mut().continue_ = new_bb();
            let body = new_bb();
            node.borrow_mut().break_ = new_bb();

            if node.borrow().init.is_some() {
                gen_stmt(node.borrow().init.clone().unwrap());
            }
            jmp(cond.clone());

            set_out(cond.clone());
            if node.borrow().cond.is_some() {
                let r = gen_expr(node.borrow().cond.clone().unwrap());
                br(r.clone(), body.clone(), node.borrow().break_.clone());
            } else {
                jmp(body.clone());
            }

            set_out(body);
            gen_stmt(node.borrow().body.clone().unwrap());
            jmp(node.borrow().continue_.clone());

            set_out(node.borrow().continue_.clone());
            if node.borrow().inc.is_some() {
                gen_expr(node.borrow().inc.clone().unwrap());
            }
            jmp(cond);

            set_out(node.borrow().break_.clone());
        }
        NodeType::DO_WHILE => {
            node.borrow_mut().continue_ = new_bb();
            let body = new_bb();
            node.borrow_mut().break_ = new_bb();

            jmp(body.clone());

            set_out(body.clone());
            gen_stmt(node.borrow().body.clone().unwrap());
            jmp(node.borrow().continue_.clone());

            set_out(node.borrow().continue_.clone());
            let r = gen_expr(node.borrow().cond.clone().unwrap());
            br(r.clone(), body, node.borrow().break_.clone());

            set_out(node.borrow().break_.clone());
        }
        NodeType::SWITCH => {
            node.borrow_mut().break_ = new_bb();
            node.borrow_mut().continue_ = new_bb();

            let r = gen_expr(node.borrow().cond.clone().unwrap());
            for c in node.borrow().cases.iter() {
                c.borrow_mut().bb = new_bb();

                let next = new_bb();
                let r2 = new_reg();
                emit(
                    IRType::EQ,
                    Some(r2.clone()),
                    Some(r.clone()),
                    Some(imm(c.borrow().val)),
                );
                br(r2.clone(), c.borrow().bb.clone(), next.clone());
                set_out(next);
            }
            jmp(node.borrow().break_.clone());

            gen_stmt(node.borrow().body.clone().unwrap());
            jmp(node.borrow().break_.clone());

            set_out(node.borrow().break_.clone());
        }
        NodeType::CASE => {
            jmp(node.borrow().bb.clone());
            set_out(node.borrow().bb.clone());
            gen_stmt(node.borrow().body.clone().unwrap());
        }
        NodeType::BREAK => {
            let target = node.borrow().target.clone().unwrap();
            jmp(target.borrow().clone().break_);
            set_out(new_bb());
        }
        NodeType::CONTINUE => {
            let target = node.borrow().target.clone().unwrap();
            jmp(target.borrow().clone().continue_);
            set_out(new_bb());
        }
        NodeType::RETURN => {
            let r = gen_expr(node.borrow().expr.clone().unwrap());
            let ir = new_ir(IRType::RETURN);
            ir.borrow_mut().r2 = Some(r);
            set_out(new_bb());
        }
        NodeType::EXPR_STMT => {
            gen_expr(node.borrow().expr.clone().unwrap());
        }
        NodeType::COMP_STMT => {
            for n in node.borrow().stmts.iter() {
                gen_stmt(n.clone());
            }
        }
        t => {
            panic!("unknown node: {:?}", t);
        }
    }
}

fn gen_param(var: &Rc<RefCell<Var>>, i: usize) {
    let ir = new_ir(IRType::STORE_ARG);
    ir.borrow_mut().var = Some(var.clone());
    ir.borrow_mut().imm = i as i32;
    ir.borrow_mut().size = var.borrow().ty.size;
    var.borrow_mut().address_taken = true;
}

pub fn gen_ir(prog: &mut Program) {
    for func in prog.funcs.iter_mut() {
        set_fn(func.clone());

        let func_node = func.borrow().node.clone();
        assert!(func_node.borrow().op == NodeType::FUNC);

        // Add an empty entry BB to make later analysis easy.
        set_out(new_bb());
        let bb = new_bb();
        jmp(bb.clone());
        set_out(bb);

        // Emit IR.
        let params = func_node.borrow().clone().params;
        for i in 0..params.len() {
            gen_param(&params[i], i)
        }

        let node_body = func_node.borrow().body.clone();
        gen_stmt(node_body.unwrap());

        // Make it always ends with a return to make later analysis easy.

        let ret_ir = new_ir(IRType::RETURN);
        ret_ir.borrow_mut().r2 = Some(imm(0));

        // Later passes shouldn't need the AST, so make it explicit.
        func.borrow_mut().node = Rc::new(RefCell::new(alloc_node()));
    }
}
