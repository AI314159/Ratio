use inkwell::context::Context;
use inkwell::builder::Builder;
use inkwell::module::Module;
use inkwell::values::{PointerValue, BasicValueEnum, BasicValue};
use inkwell::AddressSpace;
use std::collections::HashMap;
use crate::common::{Stmt, Expr, Token};

#[derive(Clone)]
enum VarKind<'ctx> {
    Int(PointerValue<'ctx>),
    Str(PointerValue<'ctx>),
}

pub fn generate_module<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    ast: &Stmt,
) {
    // Declare printf so we can use it
    let i8ptr_type = context.ptr_type(inkwell::AddressSpace::default());
    let printf_type = context.i32_type().fn_type(&[i8ptr_type.into()], true);
    module.add_function("printf", printf_type, None);

    if let Stmt::Function { name: _, body } = ast {
        let i32_type = context.i32_type();
        let fn_type = i32_type.fn_type(&[], false);
        let function = module.add_function("main", fn_type, None);
        let entry = context.append_basic_block(function, "entry");
        builder.position_at_end(entry);

        // Predefined format strings for printf
        let fmt_int = builder.build_global_string_ptr("%ld\n", "fmt_int").expect("global string").as_pointer_value();
        let fmt_str = builder.build_global_string_ptr("%s\n", "fmt_str").expect("global string").as_pointer_value();

        let mut variables: HashMap<String, VarKind> = HashMap::new();
        let mut string_literals: HashMap<String, PointerValue> = HashMap::new();

        for stmt in body {
            codegen_stmt(
                context,
                module,
                builder,
                stmt,
                &mut variables,
                &mut string_literals,
                fmt_int,
                fmt_str,
            );
        }

        builder.build_return(Some(&i32_type.const_int(0, false))).expect("return");
    }
}

fn codegen_stmt<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    stmt: &Stmt,
    variables: &mut HashMap<String, VarKind<'ctx>>,
    string_literals: &mut HashMap<String, PointerValue<'ctx>>,
    fmt_int: PointerValue<'ctx>,
    fmt_str: PointerValue<'ctx>,
) {
    match stmt {
        Stmt::VariableDecl { name, type_name, value } => {
            match type_name.as_str() {
                "int" | "bool" => {
                    let val = codegen_expr(context, module, builder, value, variables, string_literals, fmt_int, fmt_str);
                    let ptr = builder.build_alloca(context.i64_type(), name).unwrap();
                    builder.build_store(ptr, val.into_int_value()).expect("store int");
                    variables.insert(name.clone(), VarKind::Int(ptr));
                }
                _ => {
                    // Assume string type.
                    // This shouldn't be reachable because of the parser
                    let val = codegen_expr(context, module, builder, value, variables, string_literals, fmt_int, fmt_str);
                    let ptr = builder.build_alloca(context.ptr_type(AddressSpace::default()), name).unwrap();
                    builder.build_store(ptr, val.into_pointer_value()).expect("store ptr");
                    variables.insert(name.clone(), VarKind::Str(ptr));
                }
            }
        }
        Stmt::Assignment { name, value } => {
            let var_kind = variables.get(name).cloned();
            if let Some(var) = var_kind {
                match var {
                    VarKind::Int(ptr) => {
                        let val = codegen_expr(context, module, builder, value, variables, string_literals, fmt_int, fmt_str);
                        builder.build_store(ptr, val.into_int_value()).expect("store int");
                    }
                    VarKind::Str(ptr) => {
                        let val = codegen_expr(context, module, builder, value, variables, string_literals, fmt_int, fmt_str);
                        builder.build_store(ptr, val.into_pointer_value()).expect("store ptr");
                    }
                }
            }
        }
        Stmt::ExprStmt(expr) => {
            codegen_expr(context, module, builder, expr, variables, string_literals, fmt_int, fmt_str);
        }
        Stmt::IfStatement { condition, body, else_body } => {
            let parent = builder.get_insert_block().unwrap().get_parent().unwrap();
            let then_bb = context.append_basic_block(parent, "then");
            let else_bb = context.append_basic_block(parent, "else");
            let merge_bb = context.append_basic_block(parent, "ifcont");

            let cond_val = codegen_expr(context, module, builder, condition, variables, string_literals, fmt_int, fmt_str);

            let cond_bool = cond_val.into_int_value();
            builder.build_conditional_branch(cond_bool, then_bb, else_bb).unwrap();

            // Then
            builder.position_at_end(then_bb);
            for stmt in body {
                codegen_stmt(context, module, builder, stmt, variables, string_literals, fmt_int, fmt_str);
            }
            builder.build_unconditional_branch(merge_bb).unwrap();

            // Else
            builder.position_at_end(else_bb);
            if let Some(else_body) = else_body {
                for stmt in else_body {
                    codegen_stmt(context, module, builder, stmt, variables, string_literals, fmt_int, fmt_str);
                }
            }
            builder.build_unconditional_branch(merge_bb).unwrap();

            builder.position_at_end(merge_bb);
        }
        Stmt::While { condition, body } => {
            let parent = builder.get_insert_block().unwrap().get_parent().unwrap();
            let cond_bb = context.append_basic_block(parent, "while.cond");
            let body_bb = context.append_basic_block(parent, "while.body");
            let after_bb = context.append_basic_block(parent, "while.after");

            builder.build_unconditional_branch(cond_bb).unwrap();
            builder.position_at_end(cond_bb);
            let cond_val = codegen_expr(context, module, builder, condition, variables, string_literals, fmt_int, fmt_str);

            let cond_bool = cond_val.into_int_value();
            builder.build_conditional_branch(cond_bool, body_bb, after_bb).unwrap();

            builder.position_at_end(body_bb);
            for stmt in body {
                codegen_stmt(context, module, builder, stmt, variables, string_literals, fmt_int, fmt_str);
            }
            builder.build_unconditional_branch(cond_bb).unwrap();

            builder.position_at_end(after_bb);
        }
        _ => {}
    }
}

fn codegen_expr<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    expr: &Expr,
    variables: &mut HashMap<String, VarKind<'ctx>>,
    string_literals: &mut HashMap<String, PointerValue<'ctx>>,
    fmt_int: PointerValue<'ctx>,
    fmt_str: PointerValue<'ctx>,
) -> BasicValueEnum<'ctx> {
    match expr {
        Expr::Call { callee, args } => {
            if callee == "print" {
                let arg = &args[0];
                match arg {
                    Expr::StringLiteral(s) => {
                        let str_ptr = builder.build_global_string_ptr(&s, "str").expect("global string").as_pointer_value();
                        let printf = module.get_function("printf").unwrap();
                        builder.build_call(printf, &[fmt_str.into(), str_ptr.into()], "").unwrap();
                        context.i64_type().const_int(0, false).into()
                    }
                    Expr::Variable(name) => {
                        if let Some(VarKind::Int(ptr)) = variables.get(name) {
                            let val = builder.build_load(context.i64_type(),*ptr, name).unwrap().into_int_value();
                            let printf = module.get_function("printf").unwrap();
                            builder.build_call(printf, &[fmt_int.into(), val.into()], "").unwrap();
                        }
                        context.i64_type().const_int(0, false).into()
                    }
                    _ => context.i64_type().const_int(0, false).into(),
                }
            } else {
                context.i64_type().const_int(0, false).into()
            }
        }
        Expr::Variable(name) => {
            if let Some(VarKind::Int(ptr)) = variables.get(name) {
                builder.build_load(context.i64_type(), *ptr, name).unwrap().into()
            } else {
                context.i64_type().const_int(0, false).into()
            }
        }
        Expr::StringLiteral(s) => {
            builder.build_global_string_ptr(&s, "str").expect("global string").as_pointer_value().into()
        }
        Expr::IntegerLiteral(n) => context.i64_type().const_int(*n as u64, false).into(),
        Expr::BooleanLiteral(b) => context.i64_type().const_int(if *b { 1 } else { 0 }, false).into(),
        Expr::BinaryOperator { operator, left, right } => {
            let l = codegen_expr(context, module, builder, left, variables, string_literals, fmt_int, fmt_str).into_int_value();
            let r = codegen_expr(context, module, builder, right, variables, string_literals, fmt_int, fmt_str).into_int_value();
            match operator.as_str() {
                "+" => builder.build_int_add(l, r, "addtmp").unwrap().into(),
                "-" => builder.build_int_sub(l, r, "subtmp").unwrap().into(),
                "*" => builder.build_int_mul(l, r, "multmp").unwrap().into(),
                "/" => builder.build_int_signed_div(l, r, "divtmp").unwrap().into(),
                _ => context.i64_type().const_int(0, false).into(),
            }
        }
        Expr::BooleanComparison { lvalue, operator, rvalue } => {
            let l = codegen_expr(context, module, builder, lvalue, variables, string_literals, fmt_int, fmt_str).into_int_value();
            let r = codegen_expr(context, module, builder, rvalue, variables, string_literals, fmt_int, fmt_str).into_int_value();
            let pred = match operator {
                Token::Equality => inkwell::IntPredicate::EQ,
                Token::NotEqual => inkwell::IntPredicate::NE,
                Token::LessThan => inkwell::IntPredicate::SLT,
                Token::LessThanOrEqual => inkwell::IntPredicate::SLE,
                Token::GreaterThan => inkwell::IntPredicate::SGT,
                Token::GreaterThanOrEqual => inkwell::IntPredicate::SGE,
                _ => inkwell::IntPredicate::EQ,
            };
            builder.build_int_compare(pred, l, r, "cmptmp").unwrap().into()
        }
    }
}
