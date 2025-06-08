use inkwell::AddressSpace;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::BasicTypeEnum;
use inkwell::values::PointerValue;
use std::collections::HashMap;

use crate::common::{Expr, Program, Stmt, Type};
use inkwell::types::BasicType;

mod expr;
mod stmt;

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
        let arg_types: Vec<_> = ext
            .args
            .iter()
            .map(|(_, t)| match t {
                Type::Int | Type::Bool => context.i64_type().as_basic_type_enum(),
                _ => context
                    .ptr_type(inkwell::AddressSpace::default())
                    .as_basic_type_enum(),
            })
            .collect();
        let fn_arg_types: Vec<_> = arg_types.iter().map(|t| (*t).into()).collect();
        let (fn_type, ret_type) = if ext.name == "exit" || ext.return_type == "" {
            (context.void_type().fn_type(&fn_arg_types, false), None)
        } else {
            (
                context.i64_type().fn_type(&fn_arg_types, false),
                Some(context.i64_type().as_basic_type_enum()),
            )
        };
        let func = module.add_function(&ext.name, fn_type, None);
        function_table.insert(
            ext.name.clone(),
            FnSig {
                func,
                arg_types,
                ret_type,
            },
        );
    }

    for func in &program.functions {
        if let Stmt::Function { name, args, .. } = func {
            let arg_types: Vec<_> = args
                .iter()
                .map(|(_, t)| match t {
                    Type::Int | Type::Bool => context.i64_type().as_basic_type_enum(),
                    _ => context
                        .ptr_type(inkwell::AddressSpace::default())
                        .as_basic_type_enum(),
                })
                .collect();
            let fn_arg_types: Vec<_> = arg_types.iter().map(|t| (*t).into()).collect();
            let ret_type = Some(context.i64_type().as_basic_type_enum());
            let fn_type = ret_type.unwrap().fn_type(&fn_arg_types, false);
            let func_val = module.add_function(name, fn_type, None);
            function_table.insert(
                name.clone(),
                FnSig {
                    func: func_val,
                    arg_types,
                    ret_type,
                },
            );
        }
    }

    for func in &program.functions {
        if let Stmt::Function {
            name,
            args,
            body,
            return_expr,
        } = func
        {
            let fn_sig = function_table.get(name).unwrap();
            let function = fn_sig.func;
            let entry = context.append_basic_block(function, "entry");
            builder.position_at_end(entry);

            let fmt_int = builder
                .build_global_string_ptr("%ld\n", "fmt_int")
                .expect("global string")
                .as_pointer_value();
            let fmt_str = builder
                .build_global_string_ptr("%s\n", "fmt_str")
                .expect("global string")
                .as_pointer_value();

            let mut variables: HashMap<String, VarKind> = HashMap::new();
            let mut string_literals: HashMap<String, PointerValue> = HashMap::new();

            for (i, (arg_name, arg_type)) in args.iter().enumerate() {
                let llvm_arg = function.get_nth_param(i as u32).unwrap();
                let ptr = match arg_type {
                    Type::Int | Type::Bool => {
                        builder.build_alloca(context.i64_type(), arg_name).unwrap()
                    }
                    _ => builder
                        .build_alloca(context.ptr_type(AddressSpace::default()), arg_name)
                        .unwrap(),
                };
                builder.build_store(ptr, llvm_arg).unwrap();
                match arg_type {
                    Type::Int | Type::Bool => {
                        variables.insert(arg_name.clone(), VarKind::Int(ptr));
                    }
                    _ => {
                        variables.insert(arg_name.clone(), VarKind::Str(ptr));
                    }
                }
            }

            let mut did_return = false;
            for stmt in body {
                if let Stmt::Return(expr) = stmt {
                    let ret_val = expr::codegen_expr(
                        context,
                        module,
                        builder,
                        expr,
                        &mut variables,
                        &mut string_literals,
                        fmt_int,
                        fmt_str,
                        &function_table,
                    );
                    builder.build_return(Some(&ret_val)).expect("return");
                    did_return = true;
                    break;
                } else if let Stmt::ExprStmt(Expr::Call { callee, .. }) = stmt {
                    if callee == "exit" {
                        stmt::codegen_stmt(
                            context,
                            module,
                            builder,
                            stmt,
                            &mut variables,
                            &mut string_literals,
                            fmt_int,
                            fmt_str,
                            &function_table,
                        );
                        // Don't emit a return after exit()
                        did_return = true;
                        break;
                    } else {
                        stmt::codegen_stmt(
                            context,
                            module,
                            builder,
                            stmt,
                            &mut variables,
                            &mut string_literals,
                            fmt_int,
                            fmt_str,
                            &function_table,
                        );
                    }
                } else {
                    stmt::codegen_stmt(
                        context,
                        module,
                        builder,
                        stmt,
                        &mut variables,
                        &mut string_literals,
                        fmt_int,
                        fmt_str,
                        &function_table,
                    );
                }
            }
            if !did_return {
                if let Some(expr) = return_expr {
                    let ret_val = expr::codegen_expr(
                        context,
                        module,
                        builder,
                        expr,
                        &mut variables,
                        &mut string_literals,
                        fmt_int,
                        fmt_str,
                        &function_table,
                    );
                    builder.build_return(Some(&ret_val)).expect("return");
                } else {
                    builder
                        .build_return(Some(&context.i64_type().const_int(0, false)))
                        .expect("return");
                }
            }
        }
    }
}
