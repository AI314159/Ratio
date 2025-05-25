use inkwell::context::Context;
use inkwell::builder::Builder;
use inkwell::module::Module;
use inkwell::values::{PointerValue, BasicValueEnum, BasicValue};
use inkwell::AddressSpace;
use std::collections::HashMap;
use inkwell::types::BasicTypeEnum;

use crate::common::{Stmt, Expr, Token, Program};
use inkwell::types::BasicType;

#[derive(Clone)]
enum VarKind<'ctx> {
    Int(PointerValue<'ctx>),
    Str(PointerValue<'ctx>),
}

pub struct FnSig<'ctx> {
    pub func: inkwell::values::FunctionValue<'ctx>,
    pub arg_types: Vec<BasicTypeEnum<'ctx>>,
    pub ret_type: Option<BasicTypeEnum<'ctx>>,
}

pub fn generate_module<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    program: &Program,
) {
    // Declare printf so we can use it
    let i8ptr_type = context.ptr_type(inkwell::AddressSpace::default());
    let printf_type = context.i32_type().fn_type(&[i8ptr_type.into()], true);
    module.add_function("printf", printf_type, None);

    let mut function_table = std::collections::HashMap::new();

    for ext in &program.externs {
        let arg_types: Vec<_> = ext.args.iter().map(|(_, t)| match t.as_str() {
            "int" | "bool" => context.i64_type().as_basic_type_enum(),
            _ => context.ptr_type(inkwell::AddressSpace::default()).as_basic_type_enum(),
        }).collect();
        let fn_arg_types: Vec<_> = arg_types.iter().map(|t| (*t).into()).collect();
        let (fn_type, ret_type) = if ext.name == "exit" || ext.return_type == "" {
            (context.void_type().fn_type(&fn_arg_types, false), None)
        } else {
            (context.i64_type().fn_type(&fn_arg_types, false), Some(context.i64_type().as_basic_type_enum()))
        };
        let func = module.add_function(&ext.name, fn_type, None);
        function_table.insert(ext.name.clone(), FnSig { func, arg_types, ret_type });
    }

    for func in &program.functions {
        if let Stmt::Function { name, args, .. } = func {
            let arg_types: Vec<_> = args.iter().map(|(_, t)| match t.as_str() {
                "int" | "bool" => context.i64_type().as_basic_type_enum(),
                _ => context.ptr_type(inkwell::AddressSpace::default()).as_basic_type_enum(),
            }).collect();
            let fn_arg_types: Vec<_> = arg_types.iter().map(|t| (*t).into()).collect();
            let ret_type = Some(context.i64_type().as_basic_type_enum());
            let fn_type = ret_type.unwrap().fn_type(&fn_arg_types, false);
            let func_val = module.add_function(name, fn_type, None);
            function_table.insert(name.clone(), FnSig { func: func_val, arg_types, ret_type });
        }
    }

    for func in &program.functions {
        if let Stmt::Function { name, args, body, return_expr } = func {
            let fn_sig = function_table.get(name).unwrap();
            let function = fn_sig.func;
            let entry = context.append_basic_block(function, "entry");
            builder.position_at_end(entry);

            let fmt_int = builder.build_global_string_ptr("%ld\n", "fmt_int").expect("global string").as_pointer_value();
            let fmt_str = builder.build_global_string_ptr("%s\n", "fmt_str").expect("global string").as_pointer_value();

            let mut variables: HashMap<String, VarKind> = HashMap::new();
            let mut string_literals: HashMap<String, PointerValue> = HashMap::new();

            for (i, (arg_name, arg_type)) in args.iter().enumerate() {
                let llvm_arg = function.get_nth_param(i as u32).unwrap();
                let ptr = match arg_type.as_str() {
                    "int" | "bool" => builder.build_alloca(context.i64_type(), arg_name).unwrap(),
                    _ => builder.build_alloca(context.ptr_type(AddressSpace::default()), arg_name).unwrap(),
                };
                builder.build_store(ptr, llvm_arg).unwrap();
                match arg_type.as_str() {
                    "int" | "bool" => { variables.insert(arg_name.clone(), VarKind::Int(ptr)); },
                    _ => { variables.insert(arg_name.clone(), VarKind::Str(ptr)); },
                }
            }

            let mut did_return = false;
            for stmt in body {
                if let Stmt::Return(expr) = stmt {
                    let ret_val = codegen_expr(context, module, builder, expr, &mut variables, &mut string_literals, fmt_int, fmt_str, &function_table);
                    builder.build_return(Some(&ret_val)).expect("return");
                    did_return = true;
                    break;
                } else if let Stmt::ExprStmt(Expr::Call { callee, .. }) = stmt {
                    if callee == "exit" {
                        codegen_stmt(context, module, builder, stmt, &mut variables, &mut string_literals, fmt_int, fmt_str, &function_table);
                        // Don't emit a return after exit()
                        did_return = true;
                        break;
                    } else {
                        codegen_stmt(context, module, builder, stmt, &mut variables, &mut string_literals, fmt_int, fmt_str, &function_table);
                    }
                } else {
                    codegen_stmt(context, module, builder, stmt, &mut variables, &mut string_literals, fmt_int, fmt_str, &function_table);
                }
            }
            if !did_return {
                if let Some(expr) = return_expr {
                    let ret_val = codegen_expr(context, module, builder, expr, &mut variables, &mut string_literals, fmt_int, fmt_str, &function_table);
                    builder.build_return(Some(&ret_val)).expect("return");
                } else {
                    builder.build_return(Some(&context.i64_type().const_int(0, false))).expect("return");
                }
            }
        }
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
    function_table: &std::collections::HashMap<String, FnSig<'ctx>>,
) {
    match stmt {
        Stmt::VariableDecl { name, type_name, value } => {
            match type_name.as_str() {
                "int" | "bool" => {
                    let val = codegen_expr(context, module, builder, value, variables, string_literals, fmt_int, fmt_str, function_table);
                    let ptr = builder.build_alloca(context.i64_type(), name).unwrap();
                    builder.build_store(ptr, val.into_int_value()).expect("store int");
                    variables.insert(name.clone(), VarKind::Int(ptr));
                }
                _ => {
                    // Assume string type.
                    // This shouldn't be reachable because of the parser
                    let val = codegen_expr(context, module, builder, value, variables, string_literals, fmt_int, fmt_str, function_table);
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
                        let val = codegen_expr(context, module, builder, value, variables, string_literals, fmt_int, fmt_str, function_table);
                        builder.build_store(ptr, val.into_int_value()).expect("store int");
                    }
                    VarKind::Str(ptr) => {
                        let val = codegen_expr(context, module, builder, value, variables, string_literals, fmt_int, fmt_str, function_table);
                        builder.build_store(ptr, val.into_pointer_value()).expect("store ptr");
                    }
                }
            }
        }
        Stmt::ExprStmt(expr) => {
            codegen_expr(context, module, builder, expr, variables, string_literals, fmt_int, fmt_str, function_table);
        }
        Stmt::IfStatement { condition, body, else_body } => {
            let parent = builder.get_insert_block().unwrap().get_parent().unwrap();
            let then_bb = context.append_basic_block(parent, "then");
            let else_bb = context.append_basic_block(parent, "else");
            let merge_bb = context.append_basic_block(parent, "ifcont");

            let cond_val = codegen_expr(context, module, builder, condition, variables, string_literals, fmt_int, fmt_str, function_table);

            let cond_bool = cond_val.into_int_value();
            builder.build_conditional_branch(cond_bool, then_bb, else_bb).unwrap();

            // Then
            builder.position_at_end(then_bb);
            for stmt in body {
                codegen_stmt(context, module, builder, stmt, variables, string_literals, fmt_int, fmt_str, function_table);
            }
            builder.build_unconditional_branch(merge_bb).unwrap();

            // Else
            builder.position_at_end(else_bb);
            if let Some(else_body) = else_body {
                for stmt in else_body {
                    codegen_stmt(context, module, builder, stmt, variables, string_literals, fmt_int, fmt_str, function_table);
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
            let cond_val = codegen_expr(context, module, builder, condition, variables, string_literals, fmt_int, fmt_str, function_table);

            let cond_bool = cond_val.into_int_value();
            builder.build_conditional_branch(cond_bool, body_bb, after_bb).unwrap();

            builder.position_at_end(body_bb);
            for stmt in body {
                codegen_stmt(context, module, builder, stmt, variables, string_literals, fmt_int, fmt_str, function_table);
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
    function_table: &std::collections::HashMap<String, FnSig<'ctx>>,
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
            } else if let Some(fn_sig) = function_table.get(callee) {
                let mut arg_vals = Vec::new();
                for (i, arg) in args.iter().enumerate() {
                    let val = codegen_expr(context, module, builder, arg, variables, string_literals, fmt_int, fmt_str, function_table);

                    let expected = fn_sig.arg_types[i];
                    let casted = match expected {
                        inkwell::types::BasicTypeEnum::IntType(_) => val.into_int_value().as_basic_value_enum(),
                        inkwell::types::BasicTypeEnum::PointerType(_) => val.into_pointer_value().as_basic_value_enum(),
                        _ => val,
                    };
                    arg_vals.push(casted);
                }
                let call = builder.build_call(fn_sig.func, &arg_vals.iter().map(|v| (*v).into()).collect::<Vec<_>>(), "calltmp").unwrap();
                if fn_sig.ret_type.is_none() {
                    builder.build_unreachable().unwrap();
                    context.i64_type().const_int(0, false).into()
                } else {
                    call.try_as_basic_value().left().unwrap_or(context.i64_type().const_int(0, false).into())
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
            let l = codegen_expr(context, module, builder, left, variables, string_literals, fmt_int, fmt_str, function_table).into_int_value();
            let r = codegen_expr(context, module, builder, right, variables, string_literals, fmt_int, fmt_str, function_table).into_int_value();
            match operator.as_str() {
                "+" => builder.build_int_add(l, r, "addtmp").unwrap().into(),
                "-" => builder.build_int_sub(l, r, "subtmp").unwrap().into(),
                "*" => builder.build_int_mul(l, r, "multmp").unwrap().into(),
                "/" => builder.build_int_signed_div(l, r, "divtmp").unwrap().into(),
                _ => context.i64_type().const_int(0, false).into(),
            }
        }
        Expr::BooleanComparison { lvalue, operator, rvalue } => {
            let l = codegen_expr(context, module, builder, lvalue, variables, string_literals, fmt_int, fmt_str, function_table).into_int_value();
            let r = codegen_expr(context, module, builder, rvalue, variables, string_literals, fmt_int, fmt_str, function_table).into_int_value();
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
