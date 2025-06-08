use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::{BasicValue, BasicValueEnum, PointerValue};
use std::collections::HashMap;

use super::FnSig;
use super::VarKind;
use crate::common::{Expr, Token};

pub fn codegen_expr<'ctx>(
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
                        let str_ptr = builder
                            .build_global_string_ptr(&s, "str")
                            .expect("global string")
                            .as_pointer_value();
                        let printf = module.get_function("printf").unwrap();
                        builder
                            .build_call(printf, &[fmt_str.into(), str_ptr.into()], "")
                            .unwrap();
                        context.i64_type().const_int(0, false).into()
                    }
                    Expr::Variable(name) => {
                        if let Some(VarKind::Int(ptr)) = variables.get(name) {
                            let val = builder
                                .build_load(context.i64_type(), *ptr, name)
                                .unwrap()
                                .into_int_value();
                            let printf = module.get_function("printf").unwrap();
                            builder
                                .build_call(printf, &[fmt_int.into(), val.into()], "")
                                .unwrap();
                        }
                        context.i64_type().const_int(0, false).into()
                    }
                    _ => context.i64_type().const_int(0, false).into(),
                }
            } else if let Some(fn_sig) = function_table.get(callee) {
                let mut arg_vals = Vec::new();
                for (i, arg) in args.iter().enumerate() {
                    let val = codegen_expr(
                        context,
                        module,
                        builder,
                        arg,
                        variables,
                        string_literals,
                        fmt_int,
                        fmt_str,
                        function_table,
                    );

                    let expected = fn_sig.arg_types[i];
                    let casted = match expected {
                        inkwell::types::BasicTypeEnum::IntType(_) => {
                            val.into_int_value().as_basic_value_enum()
                        }
                        inkwell::types::BasicTypeEnum::PointerType(_) => {
                            val.into_pointer_value().as_basic_value_enum()
                        }
                        _ => val,
                    };
                    arg_vals.push(casted);
                }
                let call = builder
                    .build_call(
                        fn_sig.func,
                        &arg_vals.iter().map(|v| (*v).into()).collect::<Vec<_>>(),
                        "calltmp",
                    )
                    .unwrap();
                if fn_sig.ret_type.is_none() {
                    builder.build_unreachable().unwrap();
                    context.i64_type().const_int(0, false).into()
                } else {
                    call.try_as_basic_value()
                        .left()
                        .unwrap_or(context.i64_type().const_int(0, false).into())
                }
            } else {
                context.i64_type().const_int(0, false).into()
            }
        }
        Expr::Variable(name) => {
            if let Some(VarKind::Int(ptr)) = variables.get(name) {
                builder
                    .build_load(context.i64_type(), *ptr, name)
                    .unwrap()
                    .into()
            } else {
                context.i64_type().const_int(0, false).into()
            }
        }
        Expr::StringLiteral(s) => builder
            .build_global_string_ptr(&s, "str")
            .expect("global string")
            .as_pointer_value()
            .into(),
        Expr::IntegerLiteral(n) => context.i64_type().const_int(*n as u64, false).into(),
        Expr::BooleanLiteral(b) => context
            .i64_type()
            .const_int(if *b { 1 } else { 0 }, false)
            .into(),
        Expr::BinaryOperator {
            operator,
            left,
            right,
        } => {
            let l = codegen_expr(
                context,
                module,
                builder,
                left,
                variables,
                string_literals,
                fmt_int,
                fmt_str,
                function_table,
            )
            .into_int_value();
            let r = codegen_expr(
                context,
                module,
                builder,
                right,
                variables,
                string_literals,
                fmt_int,
                fmt_str,
                function_table,
            )
            .into_int_value();
            match operator.as_str() {
                "+" => builder.build_int_add(l, r, "addtmp").unwrap().into(),
                "-" => builder.build_int_sub(l, r, "subtmp").unwrap().into(),
                "*" => builder.build_int_mul(l, r, "multmp").unwrap().into(),
                "/" => builder.build_int_signed_div(l, r, "divtmp").unwrap().into(),
                _ => context.i64_type().const_int(0, false).into(),
            }
        }
        Expr::BooleanComparison {
            lvalue,
            operator,
            rvalue,
        } => {
            let l = codegen_expr(
                context,
                module,
                builder,
                lvalue,
                variables,
                string_literals,
                fmt_int,
                fmt_str,
                function_table,
            )
            .into_int_value();
            let r = codegen_expr(
                context,
                module,
                builder,
                rvalue,
                variables,
                string_literals,
                fmt_int,
                fmt_str,
                function_table,
            )
            .into_int_value();
            let pred = match operator {
                Token::Equality => inkwell::IntPredicate::EQ,
                Token::NotEqual => inkwell::IntPredicate::NE,
                Token::LessThan => inkwell::IntPredicate::SLT,
                Token::LessThanOrEqual => inkwell::IntPredicate::SLE,
                Token::GreaterThan => inkwell::IntPredicate::SGT,
                Token::GreaterThanOrEqual => inkwell::IntPredicate::SGE,
                _ => inkwell::IntPredicate::EQ,
            };
            builder
                .build_int_compare(pred, l, r, "cmptmp")
                .unwrap()
                .into()
        }
    }
}
